use actix_web::HttpResponse;

use crate::session_state::TypedSession;

#[tracing::instrument(
    "Logging out currently logged in user",
    skip_all
)]
pub async fn logout(
    session: TypedSession
) -> HttpResponse {
    session.purge();
    HttpResponse::Ok().finish()
}
