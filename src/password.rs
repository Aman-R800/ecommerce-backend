use anyhow::Context;
use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};

use crate::telemetry::spawn_blocking_with_tracing;

// Function to compute password hash
pub fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error>{
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
                            .hash_password(password.expose_secret().as_bytes(), &salt)
                            .map_err(|_| anyhow::anyhow!("Failed to compute password hash"))?
                            .to_string();

    Ok(SecretString::from(password_hash))
}

// Function to verify if password matches hash
pub async fn verify_password(password: SecretString, hashed_password: String) -> Result<bool, anyhow::Error>{
    let verified = spawn_blocking_with_tracing(move ||{
        let argon2 = Argon2::default();
        let hashed_password = PasswordHash::try_from(hashed_password.as_str())
                    .map_err(|_| anyhow::anyhow!("Failed to parse PasswordHash \
                            from stored hashed password"));
        match hashed_password {
            Ok(e) => {
                Ok(argon2
                    .verify_password(password.expose_secret().as_bytes(), &e)
                    .is_ok()
                )
            },

            Err(e) => {
                Err(e)
            }
        }
    })
    .await
    .context("Failed due to threadpool error")?;

    verified
}
