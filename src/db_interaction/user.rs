use std::{error::Error, fmt::Debug};

use diesel::{Connection, QueryResult};
use anyhow::Context;
use diesel::{RunQueryDsl, QueryDsl, ExpressionMethods};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use uuid::Uuid;

use crate::{models::{ConfirmationMap, User, UserProfileInfo}, password::compute_password_hash, schema::users, telemetry::spawn_blocking_with_tracing, utils::{error_fmt_chain, DbConnection}};


// Function to query user from email id
pub async fn get_user_from_email(
    mut conn: DbConnection,
    email_string: String
) -> Result<QueryResult<User>, anyhow::Error> {
    let res = spawn_blocking_with_tracing(move || {
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

    Ok(res)
}

// Error associated with inserting user to users table
#[derive(Error)]
pub enum UserInsertError{
    #[error("email field is not unique")]
    EmailNotUnique(#[from] diesel::result::Error),
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
    skip(conn)
)]
pub async fn insert_user_into_database(
    mut conn: DbConnection,
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
        status: Some("pending".to_string()),
        is_admin: false
    };

    let confirmation_id = {
        use crate::schema::users::dsl::*;
        use crate::schema::confirmation::dsl::*;
        spawn_blocking_with_tracing(move || {
            conn.transaction::<_, UserInsertError, _>(|conn| {
            
                diesel::insert_into(users)
                    .values(user)
                    .execute(conn)
                    .map_err(|e|{
                        match e {
                            diesel::result::Error::DatabaseError(
                                diesel::result::DatabaseErrorKind::UniqueViolation,
                                ref _a
                            ) => {
                                UserInsertError::EmailNotUnique(e)
                            },

                            _ => UserInsertError::UnexpectedError(anyhow::anyhow!("Unexpected diesel / database error"))
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
                    .map_err(|_| UserInsertError::UnexpectedError(anyhow::anyhow!("Unexpected diesel / database error")))?;

                Ok(id)
            })
        })
        .await
        .context("Failed due to threadpool error")
        .map_err(UserInsertError::UnexpectedError)??
    };

    
    Ok(confirmation_id)
}

#[tracing::instrument(
    "Get profile data of logged in user",
    skip(conn)
)]
pub async fn get_user_profile_info(
    mut conn: DbConnection,
    user_id: Uuid
) -> Result<UserProfileInfo, anyhow::Error>{
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

// Errors associated with inserting / updating user profile to database
#[derive(thiserror::Error)]
pub enum PostUserProfileInfoError{
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
    "posting user profile info to db",
    skip_all
)]
pub async fn post_user_profile_info(
    mut conn: DbConnection,
    new_info: UserProfileInfo,
    user_id: Uuid
) -> Result<(), PostUserProfileInfoError>{

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

#[tracing::instrument(
    "Get user_id from confirmation_id",
    skip(conn)
)]
pub async fn get_user_id_from_confirmation_id(
    confirmation_id: Uuid,
    mut conn: DbConnection
) -> Result<Uuid, anyhow::Error>{
    use crate::schema::confirmation;

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
    skip(conn)
)]
pub async fn set_status_confirm(
    user_id: Uuid,
    mut conn: DbConnection
) -> Result<(), anyhow::Error> {
    use crate::schema::users;

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
