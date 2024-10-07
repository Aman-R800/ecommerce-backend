use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;
use thiserror::Error;

use crate::{auth::extractors::IsUser, models::UserProfileInfo, schema::users, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbPool}};

#[derive(Error)]
pub enum GetProfileError {
    #[error("Unexpected Error Occured")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for GetProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for GetProfileError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::InternalServerError().body(format!("{}", self))
    }
}

#[tracing::instrument(
    "Get profile data of logged in user",
    skip(pool, uid)
)]
pub async fn get_profile(
    pool: web::Data<DbPool>,
    uid: IsUser
) -> Result<HttpResponse, GetProfileError>{
    let user_id_uuid = uid.0.clone();
    let user_profile_info = get_user_profile_info(&pool, user_id_uuid).await?;

    Ok(HttpResponse::Ok().json(user_profile_info))
}

#[tracing::instrument(
    "Get profile data of logged in user",
    skip(pool)
)]
pub async fn get_user_profile_info(
    pool: &DbPool,
    user_id: Uuid
) -> Result<UserProfileInfo, anyhow::Error>{
    let mut conn = pool.get()
                    .context("Failed to get connection from pool")?;

    Ok(spawn_blocking_with_tracing(move || {
        users::table.select((
            users::name,
            users::email,
            users::phone_number,
            users::address
        ))
        .filter(users::user_id.eq(user_id))
        .get_result::<UserProfileInfo>(&mut conn)
        .context("Failed to get UserProfileInfo from database")
    })
    .await
    .context("Failed due to threadpool error")??)
}
