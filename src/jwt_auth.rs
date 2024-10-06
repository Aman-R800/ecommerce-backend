use chrono::{Duration, Utc};
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
