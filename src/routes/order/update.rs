use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{auth::extractors::IsAdmin, db_interaction::{update_order_status, UpdateOrderStatusError}, utils::{error_fmt_chain, get_pooled_connection, DbPool}};

// Struct representing put order status form
#[derive(Deserialize, Debug)]
pub struct UpdateOrderStatusForm{
    pub order_id: Uuid,
    pub status: OrderStatus
}

// Enum representing updated order status
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus{
    Pending,
    Shipped,
    Delivered
}

// Error response associated with order status update route
#[derive(Error)]
pub enum UpdateOrderError {
    #[error("Failed due to internal server error")]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Incorrect order id given: {0}")]
    IncorrectOrderId(Uuid)
}

impl Debug for UpdateOrderError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for UpdateOrderError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let mut req_builder = match self { 
            Self::UnexpectedError(_) => HttpResponse::InternalServerError(),
            Self::IncorrectOrderId(_) => HttpResponse::BadRequest()
        };

        req_builder.body(format!("{}", self))
    }
}

#[tracing::instrument(
    "Updating order status",
    skip(pool)
)]
pub async fn update_order(
    pool: web::Data<DbPool>,
    form: web::Form<UpdateOrderStatusForm>,
    _: IsAdmin
) -> Result<HttpResponse, UpdateOrderError>{
    let conn = get_pooled_connection(&pool)
                    .await
                    .context("Failed to get connection from pool from within spawned task")?;

    update_order_status(
        conn,
        form.0.status,
        form.0.order_id
    )
    .await
    .map_err(|e| {
        match e {
            UpdateOrderStatusError::ThreadpoolError(_) => UpdateOrderError::UnexpectedError(e.into()),
            UpdateOrderStatusError::RunQueryError(_) => UpdateOrderError::UnexpectedError(e.into()),
            UpdateOrderStatusError::NoOrderIdError(r) => UpdateOrderError::IncorrectOrderId(r)
        }
    })?;

    Ok(HttpResponse::Ok().finish())
}
