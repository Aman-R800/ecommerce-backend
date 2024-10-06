use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use diesel::{Connection, ExpressionMethods, RunQueryDsl};
use serde::Deserialize;
use uuid::Uuid;

use crate::{jwt_auth::IsUser, schema::orders, telemetry::spawn_blocking_with_tracing, utils::DbPool};

#[derive(Deserialize, Debug)]
pub struct DeleteOrderJson{
    pub order_id: Uuid
}

#[tracing::instrument(
    "Deleting order by id"
    skip(pool)
)]
pub async fn delete_order(
    pool: web::Data<DbPool>,
    json: web::Json<DeleteOrderJson>,
    _: IsUser
) -> Result<HttpResponse, actix_web::Error>{
    let mut conn = pool.get().map_err(|_|{
        ErrorInternalServerError(
            anyhow::anyhow!("Failed due to internal error")
        )
    })?;

    spawn_blocking_with_tracing(move || {
        conn.transaction::<(), anyhow::Error, _>(|conn| {

            diesel::delete(orders::table)
                .filter(orders::order_id.eq(json.order_id))
                .execute(conn)
                .context("Failed to delete order")?;
            
            Ok(())
        })
    })
    .await
    .map_err(|_| ErrorInternalServerError(anyhow::anyhow!("Failed due to internal error")))?
    .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}
