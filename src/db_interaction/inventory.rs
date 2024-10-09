use std::{error::Error, fmt::Debug};

use anyhow::Context;
use diesel::{RunQueryDsl, QueryDsl};
use thiserror::Error;

use crate::{models::InventoryItem, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbConnection}};

#[tracing::instrument(
    "Getting inventory items from db",
    skip_all
)]
pub async fn get_inventory_items(
    mut conn: DbConnection,
    page: i64,
    limit: i64
) -> Result<Vec<InventoryItem>, anyhow::Error>{
    let offset_value = (page - 1) * limit;

    let res = spawn_blocking_with_tracing(move || {
        use crate::schema::inventory; 

        inventory::table
            .limit(limit)
            .offset(offset_value)
            .load::<InventoryItem>(&mut conn)
            .context("Failed to get inventory items")
    })
    .await
    .context("Failed due to threadpool error")??;

    Ok(res)
}

#[derive(Error)]
pub enum InventoryInsertError{
    #[error("Failed due to threadpool error")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed to insert into inventory table")]
    InsertError(#[from] diesel::result::Error)
}

impl Debug for InventoryInsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

#[tracing::instrument(
    "Insert an inventory item to db",
    skip_all
)]
pub async fn insert_inventory_items(
    mut conn: DbConnection,
    inventory_item: InventoryItem
) -> Result<(), InventoryInsertError> {

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
