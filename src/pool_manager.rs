use std::sync::{Arc, LazyLock};

use dashmap::DashMap;

use crate::error::DbError;
use crate::models::db_config::UormOptions;
use crate::executor::session::Session;
use crate::rdbc::pool::Pool;
#[cfg(feature = "mysql")]
use crate::rdbc_mysql::pool::MysqlPool;
use crate::executor::mapper::Mapper;
use crate::mapper_loader::{load, load_from_path};

// 全局单例（Rust 1.80+ 推荐）
pub static DB: LazyLock<PoolManager> = LazyLock::new(PoolManager::new);

/// 数据库连接池管理器
/// Manages database connection pools
pub struct PoolManager {
    pools: DashMap<String, Arc<dyn Pool>>,
}

/// 获取全局 PoolManager 实例
pub fn pool_mgr() -> &'static PoolManager {
    &DB
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
        }
    }

    /// 注册数据库连接池
    pub async fn register(&self, name: &str, options: &UormOptions<'_>) -> Result<(), DbError> {
        let scheme = options
            .conn_options
            .url
            .split("://")
            .next()
            .ok_or_else(|| DbError::InvalidDatabaseUrl("Invalid URL format".into()))?;
        if let Some(assets) = &options.assets {
            load(assets).map_err(|e| DbError::Database(e.to_string()))?;
        }
        if let Some(path) = options.assets_path {
            load_from_path(std::path::Path::new(path)).map_err(|e| DbError::Database(e.to_string()))?;
        }
        let pool: Arc<dyn Pool> = match scheme {
            #[cfg(feature = "mysql")]
            "mysql" => Arc::new(MysqlPool::connect(&options.conn_options).await?),
            _ => return Err(DbError::UnsupportedDatabaseType(scheme.into())),
        };

        self.pools.insert(name.to_string(), pool);
        Ok(())
    }

    
    /// 获取用于执行原生 SQL 查询的客户端
    pub fn session(&self, db_name: &str) -> Option<Session> {
        self.pools
            .get(db_name)
            .map(|v| Session::new(v.value().clone()))
    }
    
    /// 获取用于执行映射器操作的客户端
    pub fn mapper(&self, db_name: &str) -> Option<Mapper> {
        self.pools
            .get(db_name)
            .map(|v| Mapper::new(v.value().clone()))
    }
}
