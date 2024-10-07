use actix_web::{error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized}, web, HttpResponse};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use secrecy::SecretString;
use serde::Deserialize;
use serde_json::json;

use crate::{auth::jwt::Tokenizer, domain::user_email::UserEmail, models::User, password::verify_password, telemetry::spawn_blocking_with_tracing, utils::DbPool};


#[derive(Deserialize, Debug)]
pub struct LoginForm{
    pub email: String,
    pub password: SecretString
}

#[tracing::instrument(
    "Logging in user",
    skip(pool, tokenizer)
)]
pub async fn login(
    pool: web::Data<DbPool>,
    form: web::Form<LoginForm>,
    tokenizer: web::Data<Tokenizer>
) -> Result<HttpResponse, actix_web::Error>{
    let email = UserEmail::parse(form.0.email)
                    .map_err(ErrorBadRequest)?;


    let user_info = match get_user_info(&pool, &email).await
                                .map_err(ErrorInternalServerError)?{
        Some(p) => p,
        None => return Err(ErrorBadRequest(anyhow::anyhow!("No user registered with this email")))
    };

    match verify_password(form.0.password, user_info.password.clone()).await{
        Ok(res) => {
            if res {
                let jwt_token = tokenizer.generate_key(user_info);
                return Ok(HttpResponse::Ok().json(json!({ "access_token": jwt_token })))

            } else {
                tracing::info!("Passwords did not match");
                return Err(ErrorUnauthorized("Email or password is incorrect"))
            }
        },
        Err(e) => {
            let err = e.to_string();
            tracing::error!(err);
            return Err(ErrorInternalServerError("Failed to login"));
        }
    }
}

#[tracing::instrument(
    "Getting user info from email"
)]
pub async fn get_user_info(pool: &DbPool, email: &UserEmail) -> Result<Option<User>, anyhow::Error>{
    let mut conn = pool.get()?;
    let email_string = email.0.clone();

    let user = spawn_blocking_with_tracing(move || {
        use crate::schema::users;

        let res: QueryResult<User> = users::table.select((
            users::user_id,
            users::name,
            users::email,
            users::password,
            users::status,
            users::is_admin
        ))
        .filter(users::email.eq(email_string))
        .get_result::<User>(&mut conn);

        res
    })
    .await
    .context("Failed due to threadpool error")?;

    match user{
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            tracing::error!("{:?}", e);
            Ok(None)
        }
    }
}
