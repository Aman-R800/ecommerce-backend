use std::{error::Error, fmt::Debug};

use actix_web::{web, HttpResponse, ResponseError};
use anyhow::{anyhow, Context};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{models::User, password::compute_password_hash, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbPool}};

#[tracing::instrument(
    "User registration started",
    skip(pool)
)]
pub async fn register(form: web::Form<RegistrationForm>, pool: web::Data<DbPool>) -> Result<HttpResponse, actix_web::Error>{
    if form.password.expose_secret() != form.confirm_password.expose_secret(){
        return Err(RegisterError::PasswordNotMatching.into())
    }

    insert_user_into_database(&pool, form.0.name, form.0.email, form.0.password)
        .await
        .map_err(|e| {
            match e {
                UserInsertError::EmailNotUnique(_) => RegisterError::UserAlreadyExists(e),
                UserInsertError::UnexpectedError(_) => RegisterError::UnexpectedError(e.into())
            }
        })?;

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
) -> Result<(), UserInsertError> {

    let password_hash = spawn_blocking_with_tracing(move || {
        compute_password_hash(password)
    })
    .await
    .context("Failed due to threadpool error")
    .map_err(UserInsertError::UnexpectedError)?
    .map_err(UserInsertError::UnexpectedError)?;

    let user = User{
        user_id: Uuid::new_v4(),
        name,
        email,
        password: password_hash.expose_secret().to_string(),
    };

    let mut conn = pool.get()
                .context("Failed to get connection from pool")
                .map_err(UserInsertError::UnexpectedError)?;

    {
        use crate::schema::users::dsl::*;

       spawn_blocking_with_tracing(move || {
            diesel::insert_into(users)
                .values(user)
                .execute(&mut conn)
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
                })
        })
        .await
        .context("Failed due to threadpool error")
        .map_err(UserInsertError::UnexpectedError)??;
    }

    Ok(())
}
