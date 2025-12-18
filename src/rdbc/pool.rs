use crate::error::DbError;
use crate::rdbc::connection::Connection;
use async_trait::async_trait;
use std::sync::Arc;
use crate::models::db_type::DatabaseType;

/// Represents a connection pool or factory.
#[async_trait]
pub trait Pool: Send + Sync {
    
    fn db_type(&self) -> DatabaseType;
    
    async fn get_connection(&self) -> Result<Arc<dyn Connection>, DbError>;
    async fn close(&self) -> Result<(), DbError>;
}
