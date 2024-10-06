use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use diesel::{Connection, ExpressionMethods, RunQueryDsl};
use serde::Deserialize;
use uuid::Uuid;

use crate::{jwt_auth::IsAdmin, schema::orders, telemetry::spawn_blocking_with_tracing, utils::DbPool};

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
    let mut conn = pool.get()
        .map_err(|_| ErrorInternalServerError(anyhow::anyhow!("Failed due to internal error")))?;

    dbg!(&form.order_id);

    spawn_blocking_with_tracing(move || {
        conn.transaction::<(), anyhow::Error, _>(|conn| {
            let status = match form.status {
                OrderStatus::Pending => "pending",
                OrderStatus::Shipped => "shipped",
                OrderStatus::Delivered => "delivered"
            }.to_string();

            let affected_rows = diesel::update(orders::table)
                                    .filter(orders::order_id.eq(form.order_id))
                                    .set(orders::status.eq(status))
                                    .execute(conn)
                                    .context("Failed to update status")?;

            if affected_rows == 0 {
                return Err(anyhow::anyhow!("order_id doesn't exist"));
            }
            
            Ok(())
        })
    })
    .await
    .map_err(|_| ErrorInternalServerError(anyhow::anyhow!("Failed due to internal error")))?
    .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}
