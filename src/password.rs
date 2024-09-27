use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use secrecy::{ExposeSecret, SecretString};


pub fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error>{
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
                            .hash_password(password.expose_secret().as_bytes(), &salt)
                            .map_err(|_| anyhow::anyhow!("Failed to compute password hash"))?
                            .to_string();

    Ok(SecretString::from(password_hash))
}
