use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::extractors::IsAdmin, db_interaction::update_order_status, utils::{get_pooled_connection, DbPool}};

#[derive(Deserialize, Debug)]
pub struct UpdateOrderStatusForm{
    pub order_id: Uuid,
    pub status: OrderStatus
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus{
    Pending,
    Shipped,
    Delivered
}

#[tracing::instrument(
    "Updating order status",
    skip(pool)
)]
pub async fn update_order(
    pool: web::Data<DbPool>,
    form: web::Form<UpdateOrderStatusForm>,
    _: IsAdmin
) -> Result<HttpResponse, actix_web::Error>{
    let conn = get_pooled_connection(&pool)
                    .await
                    .map_err(|_| ErrorInternalServerError(anyhow::anyhow!("Failed due to internal error")))?;

    update_order_status(
        conn,
        form.0.status,
        form.0.order_id
    )
    .await
    .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}
