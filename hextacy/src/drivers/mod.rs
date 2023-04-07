pub mod cache;
pub mod db;
#[cfg(any(feature = "email", feature = "full"))]
pub mod email;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriverError {
    #[cfg(any(feature = "full", feature = "db", feature = "mongo"))]
    #[error("Mongo driver error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[cfg(any(feature = "full", feature = "db", feature = "postgres-diesel"))]
    #[error("Postgres pool error: {0}")]
    PgPoolConnection(String),
    #[cfg(any(feature = "full", feature = "db", feature = "postgres-diesel"))]
    #[error("Diesel error: {0}")]
    DieselResult(#[from] diesel::result::Error),
    #[cfg(any(feature = "full", feature = "db", feature = "postgres-diesel"))]
    #[error("PG Connection error: {0}")]
    PgDirectConnection(#[from] diesel::ConnectionError),

    #[cfg(any(feature = "full", feature = "db", feature = "redis"))]
    #[error("Redis pool error: {0}")]
    RdPoolConnection(String),
    #[cfg(any(feature = "full", feature = "db", feature = "redis"))]
    #[error("RD Connection error: {0}")]
    RdDirectConnection(#[from] redis::RedisError),

    #[cfg(any(feature = "full", feature = "email"))]
    #[error("Transport error: {0}")]
    Transport(#[from] lettre::transport::smtp::Error),
    #[cfg(any(feature = "full", feature = "email"))]
    #[error("Email error: {0}")]
    Email(#[from] lettre::error::Error),
}