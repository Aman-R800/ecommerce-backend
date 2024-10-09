use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::extractors::IsUser, db_interaction::delete_order_from_database, utils::{get_pooled_connection, DbPool}};

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
    let conn = get_pooled_connection(&pool)
                    .await
                    .map_err(|_|{
                        ErrorInternalServerError(
                            anyhow::anyhow!("Failed due to internal error")
                        )
                    })?;

    delete_order_from_database(conn, json.order_id)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}
