use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{auth::extractors::IsUser, db_interaction::{create_order_and_update_inventory, CreateOrderUpdateInventoryError}, utils::{error_fmt_chain, get_pooled_connection, DbPool}};

#[derive(Deserialize, Debug)]
pub struct OrderItem{
    item_id: Uuid,
    amount: i32
}

#[derive(Error)]
pub enum PostOrderError{
    #[error("Internal server error occured")]
    UnexpectedError(#[from] anyhow::Error),
    #[error("No stock available")]
    StockError
}

impl Debug for PostOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for PostOrderError{
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            Self::UnexpectedError(_) => HttpResponse::InternalServerError().body(format!("{}", self)),
            Self::StockError => HttpResponse::BadRequest().body(format!("{}", self))
        }
    }
}

#[tracing::instrument(
    "Posting order",
    skip(pool, uid)
)]
pub async fn post_order(
    pool: web::Data<DbPool>,
    order: web::Json<Vec<OrderItem>>,
    uid: IsUser
) -> Result<HttpResponse, PostOrderError> {
    let user_id = uid.0;

    let item_ids: Vec<Uuid> = order.iter()
                    .map(|item| item.item_id)
                    .collect();

    let amounts: Vec<i32> = order.iter()
                    .map(|item| item.amount)
                    .collect();
    
    let conn = get_pooled_connection(&pool)
                .await
                .context("Failed to get connection from pool from spawned task")?;

    Ok(HttpResponse::Ok().json(
        create_order_and_update_inventory(conn, item_ids, amounts, user_id)
                .await
                .map_err(|e|
                    match e {
                        CreateOrderUpdateInventoryError::ThreadpoolError(r) => PostOrderError::UnexpectedError(r.into()),
                        CreateOrderUpdateInventoryError::RunQueryError(r)=> PostOrderError::UnexpectedError(r.into()),
                        CreateOrderUpdateInventoryError::NoStockError => PostOrderError::StockError
                    }
                )?
    ))
}
