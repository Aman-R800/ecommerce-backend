use actix_web::{error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized}, web, HttpResponse};
use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use secrecy::SecretString;
use serde::Deserialize;

use crate::{domain::user_email::UserEmail, models::User, password::verify_password, session_state::TypedSession, telemetry::spawn_blocking_with_tracing, utils::DbPool};


#[derive(Deserialize, Debug)]
pub struct LoginForm{
    pub email: String,
    pub password: SecretString
}

#[tracing::instrument(
    "Logging in user",
    skip(pool, session)
)]
pub async fn login(
    pool: web::Data<DbPool>,
    form: web::Form<LoginForm>,
    session: TypedSession
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
                session.renew();
                session.insert("user_id", &user_info.user_id.to_string())
                    .context("Failed to insert associated user_id to session")
                    .map_err(ErrorInternalServerError)?;

                if user_info.is_admin{
                    session.insert("is_admin", "TRUE")
                        .context("Failed to insert admin_privilege to the session")
                        .map_err(ErrorInternalServerError)?
                }

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

    Ok(HttpResponse::Ok().body("Successfully logged in"))
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
