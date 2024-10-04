use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use diesel::{QueryDsl, RunQueryDsl};
use serde::Deserialize;

use crate::{models::InventoryItem, telemetry::spawn_blocking_with_tracing, utils::DbPool};

#[derive(Deserialize, Debug)]
pub struct GetInventoryQuery {
    page: i64,
    limit: i64
}

#[tracing::instrument(
    "Get inventory entries",
    skip(pool)
)]
pub async fn get_inventory(
    pool: web::Data<DbPool>,
    query: web::Query<GetInventoryQuery>
) -> Result<HttpResponse, actix_web::Error> {

    let inventory_items = get_inventory_items(
        &pool,
        query.0.page,
        query.0.limit
    )
    .await
    .map_err(ErrorInternalServerError)?;
    
    Ok(HttpResponse::Ok().json(inventory_items))
}

pub async fn get_inventory_items(
    pool: &DbPool,
    page: i64,
    limit: i64
) -> Result<Vec<InventoryItem>, anyhow::Error>{
    let mut conn = pool.get()?;
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

