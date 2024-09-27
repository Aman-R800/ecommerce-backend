use std::error::Error;

use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::{Pool, PooledConnection};

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
