use std::{error::Error, fmt::Debug};

use chrono::Utc;
use diesel::{Connection, JoinOnDsl};
use anyhow::Context;
use diesel::{RunQueryDsl, QueryDsl, ExpressionMethods};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{models::{Order, OrderIntermediate, OrderItemModel}, routes::order::update::OrderStatus, schema::{order_items, orders}, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbConnection}};

// Function to delete order from DB
pub async fn delete_order_from_database(
    mut conn: DbConnection,
    order_id: Uuid
) -> Result<(), anyhow::Error> {
    let res = spawn_blocking_with_tracing(move || {
        conn.transaction::<(), anyhow::Error, _>(|conn| {

            diesel::delete(orders::table)
                .filter(orders::order_id.eq(order_id))
                .execute(conn)
                .context("Failed to delete order")?;
            
            Ok(())
        })
    })
    .await
    .map_err(|_| anyhow::anyhow!("Failed due to internal error"))??;

    Ok(res)
}

#[tracing::instrument(
    "Getting order along with associated order_items",
    skip_all
)]
pub async fn get_order_with_items(
    mut conn: DbConnection,
    page: i64,
    limit: i64,
    user_id: Uuid,
    is_admin: bool
) -> Result<Vec<OrderWithItems>, anyhow::Error> {

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
        .load::<Uuid>(conn)
        .context("Failed to load order_ids")?;

    Ok(result)
}

// Struct to represent order item within OrderWithItems
#[derive(Serialize, Deserialize)]
pub struct OrderItem {
    pub item_id: Uuid,
    pub quantity: i32,
}

// Struct to represent an order (with associated items)
#[derive(Serialize, Deserialize)]
pub struct OrderWithItems {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub order_date: String,
    pub status: String,
    pub items: Vec<OrderItem>,
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
        .context("Failed to get order items by order_id")?;

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

        items.push(OrderItem{ item_id: order_intermediate.item_id, quantity: order_intermediate.quantity });
    }

    if let Some(mut order) = order_info {
        order.items = items;
        Ok(order)
    } else {
        Err(anyhow::anyhow!("No items found for order"))
    }
}

// Error associated with creating orders and decrementing inventory stock
#[derive(Error)]
pub enum CreateOrderUpdateInventoryError{
    #[error("Tokio threadpool error occured")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed to run query")]
    RunQueryError(#[from] diesel::result::Error),
    #[error("None of the requested items have Stocks available")]
    NoStockError
}

impl Debug for CreateOrderUpdateInventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

#[tracing::instrument(
    "Creating order in order table and updating inventory",
    skip_all
)]
pub async fn create_order_and_update_inventory(
    mut conn: DbConnection,
    item_ids: Vec<Uuid>,
    amounts: Vec<i32>,
    user_id: Uuid
) -> Result<Vec<Uuid>, CreateOrderUpdateInventoryError> {

    let ret: Vec<Uuid> = spawn_blocking_with_tracing(move || {
        use crate::schema::inventory;
        use crate::schema::orders;
        use crate::schema::order_items;

        conn.transaction::<Vec<Uuid>, CreateOrderUpdateInventoryError, _>(|conn|{
            let mut successful_updates = Vec::new();
            
            // Start of updating inventory of items whose requested amounts <= available stock
            for (i, item_id) in item_ids.iter().enumerate() {
                let affected_rows: usize = diesel::update(
                       inventory::table.filter(inventory::item_id.eq(*item_id))
                    )
                    .set(inventory::amount.eq(inventory::amount - amounts[i]))
                    .filter(inventory::amount.ge(amounts[i]))
                    .execute(conn)?;


                if affected_rows > 0 {
                    successful_updates.push((*item_id, amounts[i]));
                }
            }
            // End of updating inventory

            if successful_updates.len() == 0 {
                return Err(CreateOrderUpdateInventoryError::NoStockError)
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
                .execute(conn)?;

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
                    .execute(conn)?;
            }

            // End of creating order_items 

            Ok(successful_updates.iter().map(|entry| entry.0).collect())
        })
    })
    .await??;

    Ok(ret)
}

// Error associated with updating order status
#[derive(Error)]
pub enum UpdateOrderStatusError{
    #[error("Tokio threadpool error occured")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed to run query")]
    RunQueryError(#[from] diesel::result::Error),
    #[error("order_id: {0} doesn't exist")]
    NoOrderIdError(Uuid)
}

impl Debug for UpdateOrderStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

// Function to perform update order status operation
pub async fn update_order_status(
    mut conn: DbConnection,
    status: OrderStatus,
    order_id: Uuid
) -> Result<(), UpdateOrderStatusError> {

    let res = spawn_blocking_with_tracing(move || {
        conn.transaction::<(), UpdateOrderStatusError, _>(|conn| {
            let status = match status {
                OrderStatus::Pending => "pending",
                OrderStatus::Shipped => "shipped",
                OrderStatus::Delivered => "delivered"
            }.to_string();

            let affected_rows = diesel::update(orders::table)
                                    .filter(orders::order_id.eq(order_id))
                                    .set(orders::status.eq(status))
                                    .execute(conn)?;

            if affected_rows == 0 {
                return Err(UpdateOrderStatusError::NoOrderIdError(order_id))
            }
            
            Ok(())
        })
    })
    .await??;

    Ok(res)
}
