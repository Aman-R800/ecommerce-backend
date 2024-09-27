use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::{anyhow, Context};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{domain::subscriber_email::SubscriberEmail, email_client::EmailClient, models::{ConfirmationMap, User}, password::compute_password_hash, startup::BaseUrl, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbPool}};

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

    let email = match SubscriberEmail::parse(form.email.clone()){
        Ok(email) => email,
        Err(e) => return Ok(HttpResponse::BadRequest().body(e))
    };

    let confirmation_id = insert_user_into_database(&pool, form.0.name, form.0.email, form.0.password)
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

#[derive(Error)]
pub enum UserInsertError{
    #[error("email field is not unique")]
    EmailNotUnique(#[source] anyhow::Error),
    #[error("unexpected database / hashing error occured")]
    UnexpectedError(#[from] anyhow::Error)
}

impl Debug for UserInsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}

#[tracing::instrument(
    "Inserting user into the database",
    skip(pool)
)]
pub async fn insert_user_into_database(
    pool: &DbPool,
    name: String,
    email: String,
    password: SecretString
) -> Result<Uuid, UserInsertError> {

    let password_hash = spawn_blocking_with_tracing(move || {
        compute_password_hash(password)
    })
    .await
    .context("Failed due to threadpool error")
    .map_err(UserInsertError::UnexpectedError)?
    .map_err(UserInsertError::UnexpectedError)?;

    let uid = Uuid::new_v4();
    let user = User{
        user_id: uid.clone(),
        name,
        email,
        password: password_hash.expose_secret().to_string(),
        status: Some("pending".to_string())
    };

    let mut conn = pool.get()
                .context("Failed to get connection from pool")
                .map_err(UserInsertError::UnexpectedError)?;


    let confirmation_id = {
        use crate::schema::users::dsl::*;
        use crate::schema::confirmation::dsl::*;
        spawn_blocking_with_tracing(move || {
            conn.transaction::<_, anyhow::Error, _>(|conn| {
            
                diesel::insert_into(users)
                    .values(user)
                    .execute(conn)
                    .map_err(|e|{
                        match e {
                            diesel::result::Error::DatabaseError(
                                diesel::result::DatabaseErrorKind::UniqueViolation,
                                a
                            ) => {
                                UserInsertError::EmailNotUnique(anyhow::anyhow!(a.message().to_string()))
                            },

                            _ => UserInsertError::UnexpectedError(anyhow!("Unexpected diesel / database error"))
                        }
                    })?;

                let id = Uuid::new_v4();

                let conf = ConfirmationMap{
                    confirmation_id: id.clone(),
                    user_id: Some(uid)
                };

                diesel::insert_into(confirmation)
                    .values(conf)
                    .execute(conn)
                    .map_err(|_| UserInsertError::UnexpectedError(anyhow!("Unexpected diesel / database error")))?;

                Ok(id)
            })
        })
        .await
        .context("Failed due to threadpool error")
        .map_err(UserInsertError::UnexpectedError)??
    };

    
    Ok(confirmation_id)
}
