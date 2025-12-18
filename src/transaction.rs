use std::sync::Arc;
use serde::Serialize;
use crate::error::DbError;
use crate::rdbc::connection::Connection;
use crate::rdbc::pool::Pool;
use crate::rdbc::value::Value;
use crate::models::db_type::DatabaseType;
use crate::tpl::engine;
use std::collections::HashMap;

pub struct TransactionContext {
    conn: Arc<dyn Connection>,
    committed: bool,
    db_type: DatabaseType,
}

impl TransactionContext {
    pub async fn begin(pool: Arc<dyn Pool>) -> Result<Self, DbError> {
        let conn = pool.get_connection().await?;
        conn.begin().await?;
        Ok(Self { conn, committed: false, db_type: pool.db_type() })
    }

    pub async fn commit(&mut self) -> Result<(), DbError> {
        self.conn.commit().await?;
        self.committed = true;
        Ok(())
    }

    pub async fn rollback(&mut self) -> Result<(), DbError> {
        let r = self.conn.rollback().await;
        if r.is_ok() {
            self.committed = true;
        }
        r
    }

    pub async fn query<T: Serialize>(&self, sql: &str, args: &T) -> Result<Vec<HashMap<String, Value>>, DbError> {
        let (rendered_sql, params) = engine::render_template(sql, sql, args, self.db_type);
        self.conn.query(&rendered_sql, &params).await
    }

    pub async fn execute<T: Serialize>(&self, sql: &str, args: &T) -> Result<u64, DbError> {
        let (rendered_sql, params) = engine::render_template(sql, sql, args, self.db_type);
        self.conn.execute(&rendered_sql, &params).await
    }

    pub async fn last_insert_id(&self) -> Result<u64, DbError> {
        self.conn.last_insert_id().await
    }
}
 
impl Drop for TransactionContext {
    fn drop(&mut self) {
        if !self.committed {
            let conn = self.conn.clone();
            tokio::spawn(async move { let _ = conn.rollback().await; });
        }
    }
}
