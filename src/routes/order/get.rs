use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use serde::Deserialize;

use crate::auth::extractors::IsUser;
use crate::db_interaction::get_order_with_items;
use crate::utils::DbPool;

#[derive(Deserialize, Debug)]
pub struct GetOrderQuery{
    pub page: i64,
    pub limit: i64
}

#[tracing::instrument(
    "Getting list of orders",
    skip(pool, uid)
)]
pub async fn get_order(
    pool: web::Data<DbPool>,
    query: web::Query<GetOrderQuery>,
    uid: IsUser
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = uid.0;
    let is_admin = uid.1;

    let conn = pool.get()
                .map_err(|_| ErrorInternalServerError("Failed due to internal error"))?;
    
    let order = get_order_with_items(
        conn,
        query.0.page,
        query.0.limit,
        user_id,
        is_admin
    )
    .await
    .map_err(ErrorInternalServerError)?;
    
    Ok(HttpResponse::Ok().json(order))
}

