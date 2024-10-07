use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use chrono::Utc;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::extractors::IsUser, models::{Order, OrderItemModel}, telemetry::spawn_blocking_with_tracing, utils::DbPool};

#[derive(Deserialize, Debug)]
pub struct OrderItem{
    item_id: Uuid,
    amount: i32
}

#[tracing::instrument(
    "Posting order",
    skip(pool, uid)
)]
pub async fn post_order(
    pool: web::Data<DbPool>,
    order: web::Json<Vec<OrderItem>>,
    uid: IsUser
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = uid.0;

    let item_ids: Vec<Uuid> = order.iter()
                    .map(|item| item.item_id)
                    .collect();

    let amounts: Vec<i32> = order.iter()
                    .map(|item| item.amount)
                    .collect();

    return Ok(HttpResponse::Ok().json(
        create_order_and_update_inventory(&pool, item_ids, amounts, user_id)
                .await
                .map_err(ErrorInternalServerError)?
    ));
}

#[tracing::instrument(
    "Creating order in order table and updating inventory",
    skip_all
)]
pub async fn create_order_and_update_inventory(
    pool: &DbPool,
    item_ids: Vec<Uuid>,
    amounts: Vec<i32>,
    user_id: Uuid
) -> Result<Vec<Uuid>, anyhow::Error> {
    let mut conn = pool.get()?;

    let ret: Vec<Uuid> = spawn_blocking_with_tracing(move || {
        use crate::schema::inventory;
        use crate::schema::orders;
        use crate::schema::order_items;

        conn.transaction::<Vec<Uuid>, anyhow::Error, _>(|conn|{
            let mut successful_updates = Vec::new();
            
            // Start of updating inventory of items whose requested amounts <= available stock
            for (i, item_id) in item_ids.iter().enumerate() {
                let affected_rows: usize = diesel::update(
                       inventory::table.filter(inventory::item_id.eq(*item_id))
                    )
                    .set(inventory::amount.eq(inventory::amount - amounts[i]))
                    .filter(inventory::amount.ge(amounts[i]))
                    .execute(conn)
                    .context("Failed to update inventory value")?;


                if affected_rows > 0 {
                    successful_updates.push((*item_id, amounts[i]));
                }
            }
            // End of updating inventory

            if successful_updates.len() == 0 {
                return Err(anyhow::anyhow!("None of the requested items have Stocks available"))
            }

            // Start of Creating order
            
            let order = Order{
                order_id: Uuid::new_v4(),
                user_id,
                order_date: Utc::now(),
                status: "pending".to_string()
            };
            
            diesel::insert_into(orders::table)
                .values(&order)
                .execute(conn)
                .context("Failed to create order")?;

            // End of creating order
            

            // Start of creating order_item

            for (item_id, amount) in successful_updates.iter(){
                let order_item = OrderItemModel{
                    order_item_id: Uuid::new_v4(),
                    order_id: order.order_id,
                    item_id: *item_id,
                    quantity: *amount
                };

                diesel::insert_into(order_items::table)
                    .values(order_item)
                    .execute(conn)
                    .context(format!("Failed to create order_item: {}", item_id))?;
            }

            // End of creating order_items 

            Ok(successful_updates.iter().map(|entry| entry.0).collect())
        })
    })
    .await
    .context("Failed due to threadpool error")??;

    Ok(ret)
}
