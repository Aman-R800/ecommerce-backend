use std::error::Error;
use std::fmt::Debug;

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;
use thiserror::Error;

use crate::auth::extractors::IsUser;
use crate::db_interaction::get_order_with_items;
use crate::utils::{error_fmt_chain, get_pooled_connection, DbPool};

// Struct representing query parameters for get order
#[derive(Deserialize, Debug)]
pub struct GetOrderQuery{
    pub page: i64,
    pub limit: i64
}

// Error response associated with get order
#[derive(Error)]
pub enum GetOrderError{
    #[error("Failed due to internal server error")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for GetOrderError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for GetOrderError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::InternalServerError().body(format!("{}", self))
    }
}

#[tracing::instrument(
    "Getting list of orders",
    skip(pool, uid)
)]
pub async fn get_order(
    pool: web::Data<DbPool>,
    query: web::Query<GetOrderQuery>,
    uid: IsUser
) -> Result<HttpResponse, GetOrderError> {
    let user_id = uid.0;
    let is_admin = uid.1;

    let conn = get_pooled_connection(&pool)
                .await
                .context("Failed to get connection from pool from within spawned task")?;
    
    let order = get_order_with_items(
        conn,
        query.0.page,
        query.0.limit,
        user_id,
        is_admin
    )
    .await
    .context("Failed to get order with items model")?;
    
    Ok(HttpResponse::Ok().json(order))
}

