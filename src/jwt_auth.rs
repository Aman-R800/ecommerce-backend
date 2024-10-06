use actix_web::{error::ErrorUnauthorized, FromRequest};
use chrono::{Duration, Utc};
use futures_util::future::{ready, Ready};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{configuration::JWTSettings, models::User};

#[derive(Clone)]
pub struct Tokenizer{
    pub secret: SecretString,
    pub expiry_hours: u64 
}

impl Tokenizer {
    pub fn new(settings: &JWTSettings) -> Self {
        Self{
            secret: SecretString::new(settings.secret.clone().into()),
            expiry_hours: settings.expiry_hours
        }
    }

    pub fn generate_key(&self, user: User) -> String{
        let expiry = Utc::now() + Duration::hours(self.expiry_hours as i64);
        let role = if user.is_admin{
            UserRole::ADMIN
        } else {
            UserRole::USER
        };

        let claims = Claims{
            sub: user.user_id,
            exp: expiry.timestamp() as usize,
            email: user.email,
            role
        };

        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.expose_secret().as_bytes())
        )
        .unwrap()
    }

    pub fn decode_key(&self, token: String) -> Option<Claims>{
        match jsonwebtoken::decode::<Claims>(
            &token,
            &DecodingKey::from_secret(self.secret.expose_secret().as_bytes()),
            &Validation::new(Algorithm::HS256)
        ) {
            Ok(decoded_data) => Some(decoded_data.claims),
            Err(_) => None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims{
    pub sub: Uuid,
    pub exp: usize,
    pub email: String,
    pub role: UserRole
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UserRole{
    ADMIN,
    USER,
}

pub struct IsAdmin(pub Uuid);
pub struct IsUser(pub Uuid);

impl FromRequest for IsAdmin {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let tokenizer: &Tokenizer = req.app_data::<Tokenizer>().unwrap();
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
        let tokenizer: &Tokenizer = req.app_data::<Tokenizer>().unwrap();
        let auth = req.headers().get("Authorization");

        match auth {
            Some(_) => {
                let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer").collect();
                let token = split[1].trim();

                match tokenizer.decode_key(token.to_string()){
                    Some(r) => {
                        match r.role {
                            UserRole::USER => ready(Ok(IsUser(r.sub))),
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

