use actix_web::{error::ErrorUnauthorized, web, FromRequest};
use futures_util::future::{ready, Ready};
use uuid::Uuid;

use super::jwt::{Tokenizer, UserRole};

// Extractor for admin role
pub struct IsAdmin(pub Uuid);

// Extractor for user role
pub struct IsUser(pub Uuid, pub bool);

impl FromRequest for IsAdmin {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let tokenizer: &web::Data<Tokenizer> = req.app_data().unwrap();
        let auth = req.headers().get("Authorization");

        match auth {
            Some(_) => {
                let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer").collect();
                let token = split[1].trim();

                match tokenizer.decode_key(token.to_string()){
                    Some(r) => {
                        match r.role {
                            UserRole::ADMIN => ready(Ok(IsAdmin(r.sub))),
                            _ => ready(Err(ErrorUnauthorized("Unauthorized Role")))
                        }
                    },
                    None => ready(Err(ErrorUnauthorized("Invalid Token")))
                }
            },
            None => ready(Err(ErrorUnauthorized("Invalid token")))
        }
    }
}


impl FromRequest for IsUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let tokenizer: &web::Data<Tokenizer> = req.app_data().unwrap();
        let auth = req.headers().get("Authorization");

        match auth {
            Some(_) => {
                let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer").collect();
                let token = split[1].trim();

                match tokenizer.decode_key(token.to_string()){
                    Some(r) => {
                        match r.role {
                            UserRole::USER => ready(Ok(IsUser(r.sub, false))),
                            UserRole::ADMIN => ready(Ok(IsUser(r.sub, true)))
                        }
                    },
                    None => ready(Err(ErrorUnauthorized("Invalid Token")))
                }
            },
            None => ready(Err(ErrorUnauthorized("Invalid token")))
        }
    }
}

