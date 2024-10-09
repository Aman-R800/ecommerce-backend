use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{auth::extractors::IsAdmin, db_interaction::insert_inventory_items, models::InventoryItem, utils::{error_fmt_chain, get_pooled_connection, DbPool}};
use crate::db_interaction::InventoryInsertError;

#[derive(Deserialize, Debug)]
pub struct InventoryForm{
    name: String,
    amount: i32,
    price: f64
}

#[derive(Error)]
pub enum PostInventoryError{
    #[error("Failed to insert item to inventory")]
    InsertInventoryError(#[from] InventoryInsertError),
    #[error("Failed due to internal server error")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for PostInventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for PostInventoryError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::InternalServerError().body(format!("{}", self))
    }
}

#[tracing::instrument(
    "Posting items to inventory",
    skip(pool)
)]
pub async fn post_inventory(
    pool: web::Data<DbPool>,
    form: web::Form<InventoryForm>,
    _: IsAdmin
) -> Result<HttpResponse, PostInventoryError>{

    let inventory_item = InventoryItem{
        item_id: Uuid::new_v4(),
        name: form.name.clone(),
        amount: Some(form.amount),
        price: Some(form.price)
    };

    let conn = get_pooled_connection(&pool)
                .await
                .context("Failed to get connection from pool from within spawned task")?;

    insert_inventory_items(conn, inventory_item).await?;

    Ok(HttpResponse::Ok().finish())
}
