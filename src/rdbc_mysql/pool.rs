use crate::rdbc::pool::Pool;
use crate::rdbc::connection::Connection;
use mysql_async::Pool as MySqlPoolInternal;
use async_trait::async_trait;
use std::sync::Arc;
use crate::error::DbError;
use crate::models::db_config::ConnectionOptions;
use crate::rdbc_mysql::connection::MysqlConnection;
use crate::models::db_type::DatabaseType;

pub struct MysqlPool {
    pool: MySqlPoolInternal,
}

impl MysqlPool {
    pub async fn connect(options: &ConnectionOptions) -> Result<Self, DbError> {
        let opts = mysql_async::Opts::from_url(&options.url).map_err(|e| DbError::Database(e.to_string()))?;
        let pool = MySqlPoolInternal::new(opts);
        Ok(Self { pool })
    }
}

#[async_trait]
impl Pool for MysqlPool {
    fn db_type(&self) -> DatabaseType {
        DatabaseType::MySql
    }
    async fn get_connection(&self) -> Result<Arc<dyn Connection>, DbError> {
        let conn = self.pool.get_conn().await.map_err(|e| DbError::Database(e.to_string()))?;
        Ok(Arc::new(MysqlConnection::new(conn)))
    }

    // async fn execute(&self, sql: &str, args: &[Value]) -> Result<u64, DbError> {
    //     let conn = self.get_connection().await?;
    //     conn.execute(sql, args).await
    // }

    async fn close(&self) -> Result<(), DbError> {
        todo!()
    }
}
