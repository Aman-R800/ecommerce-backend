use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::extractors::IsUser, db_interaction::create_order_and_update_inventory, utils::{get_pooled_connection, DbPool}};

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
    
    let conn = get_pooled_connection(&pool)
                .await
                .context("Failed to get connection from pool from spawned task")
                .map_err(ErrorInternalServerError)?;

    return Ok(HttpResponse::Ok().json(
        create_order_and_update_inventory(conn, item_ids, amounts, user_id)
                .await
                .map_err(ErrorInternalServerError)?
    ));
}
