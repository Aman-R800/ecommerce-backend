use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;

use crate::{db_interaction::{insert_user_into_database, UserInsertError}, domain::user_email::UserEmail, email_client::EmailClient, startup::BaseUrl, utils::{error_fmt_chain, get_pooled_connection, DbPool}};

#[tracing::instrument(
    "User registration started",
    skip(pool, email_client, base_url)
)]
pub async fn register(
    req: HttpRequest,
    form: web::Form<RegistrationForm>,
    pool: web::Data<DbPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<BaseUrl>
) -> Result<HttpResponse, actix_web::Error> {

    if form.password.expose_secret() != form.confirm_password.expose_secret(){
        return Err(RegisterError::PasswordNotMatching.into())
    }

    let email = match UserEmail::parse(form.email.clone()){
        Ok(email) => email,
        Err(e) => return Ok(HttpResponse::BadRequest().body(e))
    };

    let conn = get_pooled_connection(&pool)
                .await
                .context("Failed to get connection from pool from within spawned task")
                .map_err(RegisterError::UnexpectedError)?;

    let confirmation_id = insert_user_into_database(conn, form.0.name, form.0.email, form.0.password)
        .await
        .map_err(|e| {
            match e {
                UserInsertError::EmailNotUnique(_) => RegisterError::UserAlreadyExists(e),
                UserInsertError::UnexpectedError(_) => RegisterError::UnexpectedError(e.into())
            }
        })?;

    let conf_link = format!("{}confirm?id={}", base_url.0, confirmation_id);

    email_client.send_email(
        &email,
        "Confirmation email",
        "Click the link to confirm your ecomm account",
        &format!("Click to confirm: {}", conf_link)
    ).await
    .map_err(|_| RegisterError::UnexpectedError(anyhow::anyhow!("Failed to send confirmation email")))?;
    

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize, Debug)]
pub struct RegistrationForm{
    email: String,
    name: String,
    password: SecretString,
    confirm_password: SecretString
}

#[derive(Error)]
enum RegisterError{
    #[error("the password and confirm passwords don't match")]
    PasswordNotMatching,
    #[error("user already exists")]
    UserAlreadyExists(#[from] UserInsertError),
    #[error("unexpected error occured")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

impl ResponseError for RegisterError{
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::BadRequest().body(format!("{}", self))
    }
}
