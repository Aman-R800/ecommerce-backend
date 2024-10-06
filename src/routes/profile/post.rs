use std::fmt::Debug;
use std::error::Error;

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use diesel::{ExpressionMethods, RunQueryDsl};
use serde::Deserialize;
use uuid::Uuid;

use crate::{domain::{phone_number::PhoneNumberDomain, user_email::UserEmail}, models::UserProfileInfo, session_state::TypedSession, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbPool}};

use super::get_user_profile_info;

#[derive(Deserialize)]
pub struct ProfileForm{
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>
}

#[derive(thiserror::Error)]
pub enum PostProfileError{
    #[error("{0}")]
    InvalidEmailOrPhoneNumber(#[source] anyhow::Error),
    #[error("Email not unique")]
    EmailNotUnique(#[source] PostUserProfileInfoError),
    #[error("Unexpected error occured")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for PostProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for PostProfileError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::InternalServerError().body(format!("{}", self))
    }
}

#[tracing::instrument(
    "Posting user profile info",
    skip_all
)]
pub async fn post_profile(
    pool: web::Data<DbPool>,
    form: web::Form<ProfileForm>,
    session: TypedSession
) -> Result<HttpResponse, PostProfileError>{
    let user_id = Uuid::parse_str(&{
        match session.get("user_id")
                .context("Failed to get user id")?
        {
            Some(uid) => uid,
            None => return Err(anyhow::anyhow!("No user_id found").into())
        }
    }).unwrap();

    let info = get_user_profile_info(&pool, user_id.clone()).await?;
    let new_info = substitute_old_info_with_new(info, form.0)
                        .map_err(PostProfileError::InvalidEmailOrPhoneNumber)?;
    
    post_user_profile_info(&pool, new_info, user_id).await
        .map_err(|e|{
            match e {
                PostUserProfileInfoError::QueryError(_) => PostProfileError::EmailNotUnique(e),
                _ => PostProfileError::UnexpectedError(e.into())
            }
        })?;
    
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    "Updating old info with new info",
    skip_all
)]
pub fn substitute_old_info_with_new(
    mut current_info: UserProfileInfo,
    new_info: ProfileForm
) -> Result<UserProfileInfo, anyhow::Error>{

    if let Some(email) = new_info.email{
        UserEmail::parse(email.clone())
            .map_err(|e| {
                anyhow::anyhow!(e)
            })?;
        current_info.email = email.clone();
    }

    if let Some(name) = new_info.name{
        current_info.name = name.clone()
    }

    current_info.phone_number = match new_info.phone_number{
        Some(number) => {
            Some(PhoneNumberDomain::parse(number)
                    .map_err(|e|{
                        anyhow::anyhow!(e)
                    })?.inner())
        },
        None => None
    };

    current_info.address = new_info.address;
    
    Ok(current_info)
}

#[derive(thiserror::Error)]
pub enum PostUserProfileInfoError{
    #[error("Failed to get connection from pool")]
    DbPoolError(#[from] r2d2::Error),
    #[error("Failed due to threadpool error")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed due to database error")]
    QueryError(#[from] diesel::result::Error)
}

impl Debug for PostUserProfileInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

#[tracing::instrument(
    "posting user profile info to db"
)]
pub async fn post_user_profile_info(
    pool: &DbPool,
    new_info: UserProfileInfo,
    user_id: Uuid
) -> Result<(), PostUserProfileInfoError>{

    let mut conn = pool.get()?;

    spawn_blocking_with_tracing(move || {
        use crate::schema::users;
        diesel::update(users::table)
            .set((
                users::email.eq(new_info.email),
                users::name.eq(new_info.name),
                users::phone_number.eq(new_info.phone_number),
                users::address.eq(new_info.address)
            ))
            .filter(users::user_id.eq(user_id))
            .execute(&mut conn)
    })
    .await??;
    
    Ok(())
}
