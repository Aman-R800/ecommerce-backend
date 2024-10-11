use actix_web::{error::ErrorInternalServerError, web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::{db_interaction::{get_user_id_from_confirmation_id, set_status_confirm}, utils::DbPool};

// Struct representing query parameter for confirmation endpoint
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

    let conn = pool.get()
                .map_err(|_|ErrorInternalServerError(anyhow::anyhow!("Failed to get connection from pool from within spawned task")))?;

    let user_id = match get_user_id_from_confirmation_id(form.0.id, conn).await {
        Ok(id) => id,
        Err(e) => {
            return Err(ErrorInternalServerError(e))
        }
    };

    let conn = pool.get()
                .map_err(|_|ErrorInternalServerError(anyhow::anyhow!("Failed to get connection from pool from within spawned task")))?;

    if let Err(e) = set_status_confirm(user_id, conn).await{
        return Err(ErrorInternalServerError(e))
    };

    Ok(HttpResponse::Ok().body("confirmed subscription"))
}
