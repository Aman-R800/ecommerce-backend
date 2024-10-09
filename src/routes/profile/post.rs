use std::fmt::Debug;
use std::error::Error;

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use serde::Deserialize;

use crate::{auth::extractors::IsUser, db_interaction::{post_user_profile_info, PostUserProfileInfoError}, domain::{phone_number::PhoneNumberDomain, user_email::UserEmail}, models::UserProfileInfo, utils::{error_fmt_chain, DbPool}};
use crate::db_interaction::get_user_profile_info;

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
    uid: IsUser
) -> Result<HttpResponse, PostProfileError>{
    let user_id = uid.0;

    let conn = pool.get()
                .context("Failed to get connection from pool from within spawned task")?;

    let info = get_user_profile_info(conn, user_id.clone()).await?;
    let new_info = substitute_old_info_with_new(info, form.0)
                        .map_err(PostProfileError::InvalidEmailOrPhoneNumber)?;

    let conn = pool.get()
                .context("Failed to get connection from pool from within spawned task")?;
    
    post_user_profile_info(conn, new_info, user_id).await
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
