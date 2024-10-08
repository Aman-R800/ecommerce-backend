use std::{error::Error, fmt::Debug};

use actix_web::web;
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::{Pool, PooledConnection};
use thiserror::Error;

use crate::telemetry::spawn_blocking_with_tracing;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub fn error_fmt_chain(f: &mut std::fmt::Formatter<'_>, source: &Option<impl Error>) -> std::fmt::Result{
    if let Some(error) = source{
        write!(f, "\n\tCaused By:\n\t")?;
        write!(f, "{:?}", &error)?;
        error_fmt_chain(f, &error.source())
    } else {
        Ok(())
    }
}

pub async fn get_pooled_connection(
    pool: &web::Data<DbPool>
) -> Result<DbConnection, PoolGetError>{
    let pool_clone = pool.clone();

    let res = spawn_blocking_with_tracing(move || {
        pool_clone.get()
    })
    .await??;

    Ok(res)
}

#[derive(Error)]
pub enum PoolGetError{
    #[error("Failed due to threadpool error")]
    ThreadpoolError(#[from] tokio::task::JoinError),
    #[error("Failed to get connection from pool")]
    DbPoolError(#[from] r2d2::Error),
}

impl Debug for PoolGetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        error_fmt_chain(f, &self.source())
    }
}
