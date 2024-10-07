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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_settings() -> JWTSettings {
        JWTSettings {
            secret: "test_secret".to_string(),
            expiry_hours: 24,
        }
    }

    fn create_test_user() -> User {
        User {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            is_admin: false,
            name: "test name".to_string(),
            status: Some("pending".to_string()),
            password: "$argon2i$v=19$m=15000,t=2,p=1$YkxhSmF2N1I3MHpnSEI5ag$WmHZa82LeRXqE7NnnyDyLg".to_string()
        }
    }

    fn create_test_admin() -> User {
        User {
            user_id: Uuid::new_v4(),
            email: "admin@example.com".to_string(),
            is_admin: true,
            name: "test name".to_string(),
            status: Some("pending".to_string()),
            password: "$argon2i$v=19$m=15000,t=2,p=1$YkxhSmF2N1I3MHpnSEI5ag$WmHZa82LeRXqE7NnnyDyLg".to_string()
        }
    }

    #[test]
    fn test_tokenizer_new() {
        let settings = create_test_settings();
        let tokenizer = Tokenizer::new(&settings);
        
        assert_eq!(
            tokenizer.secret.expose_secret(),
            &settings.secret
        );
        assert_eq!(tokenizer.expiry_hours, settings.expiry_hours);
    }

    #[test]
    fn test_generate_key_for_user() {
        let tokenizer = Tokenizer::new(&create_test_settings());
        let user = create_test_user();
        let token = tokenizer.generate_key(user.clone());
        
        // Verify token can be decoded
        let claims = tokenizer.decode_key(token).expect("Failed to decode token");
        
        assert_eq!(claims.sub, user.user_id);
        assert_eq!(claims.email, user.email);
        assert!(matches!(claims.role, UserRole::USER));
    }

    #[test]
    fn test_generate_key_for_admin() {
        let tokenizer = Tokenizer::new(&create_test_settings());
        let admin = create_test_admin();
        let token = tokenizer.generate_key(admin.clone());
        
        let claims = tokenizer.decode_key(token).expect("Failed to decode token");
        
        assert_eq!(claims.sub, admin.user_id);
        assert_eq!(claims.email, admin.email);
        assert!(matches!(claims.role, UserRole::ADMIN));
    }

    #[test]
    fn test_token_expiry() {
        let tokenizer = Tokenizer::new(&create_test_settings());
        let user = create_test_user();
        let token = tokenizer.generate_key(user);
        
        let claims = tokenizer.decode_key(token).expect("Failed to decode token");
        let expected_expiry = Utc::now() + chrono::Duration::hours(24);
        
        // Allow for small time differences during test execution
        assert!(
            (claims.exp as i64 - expected_expiry.timestamp()).abs() < 5,
            "Expiry time differs significantly from expected"
        );
    }

    #[test]
    fn test_decode_invalid_token() {
        let tokenizer = Tokenizer::new(&create_test_settings());
        let result = tokenizer.decode_key("invalid_token".to_string());
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_token_with_wrong_secret() {
        // Create token with one secret
        let tokenizer1 = Tokenizer::new(&JWTSettings {
            secret: "secret1".to_string(),
            expiry_hours: 24,
        });
        let token = tokenizer1.generate_key(create_test_user());

        // Try to decode with different secret
        let tokenizer2 = Tokenizer::new(&JWTSettings {
            secret: "secret2".to_string(),
            expiry_hours: 24,
        });
        let result = tokenizer2.decode_key(token);
        assert!(result.is_none());
    }
}
