use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::Deserialize;
use uuid::Uuid;

use crate::{models::ConfirmationMap, telemetry::spawn_blocking_with_tracing, utils::DbPool};

#[derive(Deserialize, Debug)]
pub struct Confirmation{
    id: Uuid
}

#[tracing::instrument(
    "Confirm user status",
    skip(pool)
)]
pub async fn confirm(
    pool: web::Data<DbPool>,
    form: web::Query<Confirmation>
) -> Result<HttpResponse, actix_web::Error>{

    let user_id = match get_user_id(form.0.id, &pool).await {
        Ok(id) => id,
        Err(e) => {
            return Err(ErrorInternalServerError(e))
        }
    };

    if let Err(e) = set_status_confirm(user_id, &pool).await{
        return Err(ErrorInternalServerError(e))
    };

    Ok(HttpResponse::Ok().body("confirmed subscription"))
}


#[tracing::instrument(
    "Get user_id from confirmation_id",
    skip(pool)
)]
async fn get_user_id(confirmation_id: Uuid, pool: &DbPool) -> Result<Uuid, anyhow::Error>{
    use crate::schema::confirmation;

    let mut conn = pool.get()?;

    let temp: ConfirmationMap = spawn_blocking_with_tracing(move ||{
        confirmation::table
            .select((confirmation::confirmation_id, confirmation::user_id))
            .filter(confirmation::confirmation_id.eq(confirmation_id))
            .first::<ConfirmationMap>(&mut conn)
            .context("Failed to get Confirmation mapping")
    })
    .await
    .context("Failed due to threadpool error")??;

    Ok(temp.user_id.unwrap())
}


#[tracing::instrument(
    "Set user status to confirm",
    skip(pool)
)]
async fn set_status_confirm(user_id: Uuid, pool: &DbPool) -> Result<(), anyhow::Error>{
    use crate::schema::users;

    let mut conn = pool.get()?;

    spawn_blocking_with_tracing(move || {
        diesel::update(users::table)
            .filter(users::user_id.eq(user_id))
            .set(users::status.eq("confirmed"))
            .execute(&mut conn)
            .context("Failed to update user status")
    })
    .await
    .context("Failed due to threadpool error")??;

    Ok(())
}
