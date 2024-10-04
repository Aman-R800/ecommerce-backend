use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use diesel::RunQueryDsl;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{models::InventoryItem, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbPool}};

#[derive(Deserialize)]
pub struct InventoryForm{
    name: String,
    amount: i32,
    price: f64
}

#[derive(Error)]
pub enum PostInventoryError{
    #[error("Failed to insert item to inventory")]
    InsertInventoryError(#[from] InventoryInsertError)
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

pub async fn post_inventory(
    pool: web::Data<DbPool>,
    form: web::Form<InventoryForm>
) -> Result<HttpResponse, PostInventoryError>{

    let inventory_item = InventoryItem{
        item_id: Uuid::new_v4(),
        name: form.name.clone(),
        amount: Some(form.amount),
        price: Some(form.price)
    };

    insert_inventory_items(&pool, inventory_item).await?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Error)]
pub enum InventoryInsertError{
    #[error("Failed due to threadpool error")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed to get connection from pool")]
    DbPoolError(#[from] r2d2::Error),
    #[error("Failed to insert into inventory table")]
    InsertError(#[from] diesel::result::Error)
}

impl Debug for InventoryInsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

pub async fn insert_inventory_items(
    pool: &DbPool,
    inventory_item: InventoryItem
) -> Result<(), InventoryInsertError> {
    let mut conn = pool.get()?;

    spawn_blocking_with_tracing(move || {
        use crate::schema::inventory;

        diesel::insert_into(
            inventory::table
        )
        .values(inventory_item)
        .execute(&mut conn)
    })
    .await??;

    Ok(())
}
