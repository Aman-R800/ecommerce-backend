use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::utils::DbPool;


pub async fn register(form: web::Form<RegistrationForm>, pool: web::Data<DbPool>) -> Result<HttpResponse, actix_web::Error>{
    todo!()
}

#[derive(Deserialize)]
struct RegistrationForm{
    email: String,
    name: String,
    password: String
}
