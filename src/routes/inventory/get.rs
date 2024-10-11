use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use serde::Deserialize;

use crate::{db_interaction::get_inventory_items, utils::{get_pooled_connection, DbPool}};

// Struct representing query parameters for getting inventory
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

    let conn = get_pooled_connection(&pool)
                    .await
                    .map_err(|_| ErrorInternalServerError(anyhow::anyhow!("Failed due to internal server error")))?;

    let inventory_items = get_inventory_items(
        conn,
        query.0.page,
        query.0.limit
    )
    .await
    .map_err(ErrorInternalServerError)?;
    
    Ok(HttpResponse::Ok().json(inventory_items))
}

