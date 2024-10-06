use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel::prelude::*;

use crate::models::OrderIntermediate;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::DbConnection;
use crate::{session_state::TypedSession, utils::DbPool};
use crate::schema::{orders, order_items};

#[derive(Deserialize, Debug)]
pub struct GetOrderQuery{
    pub page: i64,
    pub limit: i64
}
#[derive(Serialize, Deserialize)]
pub struct OrderWithItems {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub order_date: String,
    pub status: String,
    pub items: Vec<OrderItem>,
}

#[derive(Serialize, Deserialize)]
pub struct OrderItem {
    pub item_id: Uuid,
    pub quantity: i32,
}

#[tracing::instrument(
    "Getting list of orders",
    skip(pool, session)
)]
pub async fn get_order(
    pool: web::Data<DbPool>,
    session: TypedSession,
    query: web::Query<GetOrderQuery>
) -> Result<HttpResponse, actix_web::Error> {

    let user_id: String = match session.get("user_id").map_err(ErrorInternalServerError)?{
        Some(uid) => uid,
        None => return Err(ErrorInternalServerError(anyhow::anyhow!("Unexpected error occured")))
    };

    let user_id = Uuid::parse_str(&user_id).unwrap();

    let is_admin = match session.get("is_admin").map_err(ErrorInternalServerError)?{
        Some(_) => true,
        None => false
    };

    
    let order = get_order_with_items(
        &pool,
        query.0.page,
        query.0.limit,
        user_id,
        is_admin
    )
    .await
    .map_err(ErrorInternalServerError)?;
    
    Ok(HttpResponse::Ok().json(order))
}

#[tracing::instrument(
    "Getting order along with associated order_items",
    skip_all
)]
pub async fn get_order_with_items(
    pool: &DbPool,
    page: i64,
    limit: i64,
    user_id: Uuid,
    is_admin: bool
) -> Result<Vec<OrderWithItems>, anyhow::Error> {
    let mut conn = pool.get()?;

    let res = spawn_blocking_with_tracing(move || {
        conn.transaction::<Vec<OrderWithItems>, anyhow::Error, _>(|conn|{
            let order_ids = get_order_ids(conn, is_admin, user_id, page, limit)?;
            let mut ret: Vec<OrderWithItems> = Vec::new();

            for order_id in order_ids{
                let curr = get_order_with_items_by_id(conn, order_id)?;
                ret.push(curr);
            }

            Ok(ret)
        })
    })
    .await
    .context("Failed due to threadpool error")??;

    Ok(res)
}

#[tracing::instrument(
    "Getting order ids",
    skip_all
)]
pub fn get_order_ids(
    conn: &mut DbConnection,
    is_admin: bool,
    user_id: Uuid,
    page: i64,
    limit: i64
) -> Result<Vec<Uuid>, anyhow::Error>{
    let mut query = orders::table
        .into_boxed();

    if !is_admin {
        query = query.filter(orders::user_id.eq(user_id));
    }

    let offset_value = (page - 1) * limit;

    let result = query.select(orders::order_id)
        .limit(limit)
        .offset(offset_value)
        .load::<Uuid>(conn)?;

    Ok(result)
}

#[tracing::instrument(
    "Getting joined order with order_items by id",
    skip_all
)]
pub fn get_order_with_items_by_id(conn: &mut DbConnection, target_order_id: Uuid) -> Result<OrderWithItems, anyhow::Error> {
    let results: Vec<OrderIntermediate> = orders::table
        .inner_join(order_items::table.on(order_items::order_id.eq(orders::order_id)))
        .filter(orders::order_id.eq(target_order_id))
        .select((
            orders::order_id,
            orders::user_id,
            orders::order_date,
            orders::status,
            order_items::item_id,
            order_items::quantity,
        ))
        .load::<OrderIntermediate>(conn)
        .context("Failed to get order items")?;

    // Group items by order and create OrderWithItems structure
    let mut items = Vec::new();
    let mut order_info: Option<OrderWithItems> = None;

    for order_intermediate in results {
        if order_info.is_none() {
            order_info = Some(OrderWithItems {
                order_id: order_intermediate.order_id,
                user_id: order_intermediate.user_id.unwrap(),
                order_date: order_intermediate.order_date.unwrap().to_string(),
                status: order_intermediate.status,
                items: Vec::new(),
            });
        }

        items.push(OrderItem { item_id: order_intermediate.item_id, quantity: order_intermediate.quantity });
    }

    if let Some(mut order) = order_info {
        order.items = items;
        Ok(order)
    } else {
        Err(anyhow::anyhow!("Failed to construct order"))
    }
}
