use actix_web::HttpResponse;

#[tracing::instrument(
    "Checking if api is online"
)]
pub async fn health_check() -> HttpResponse{
    HttpResponse::Ok().body("Working")
}

