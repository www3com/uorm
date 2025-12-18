#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    MySql,
    Postgres,
    Sqlite,
    Mssql,
    Oracle,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::MySql => "mysql",
            DatabaseType::Postgres => "postgres",
            DatabaseType::Sqlite => "sqlite",
            DatabaseType::Mssql => "mssql",
            DatabaseType::Oracle => "oracle",
        }
    }
}
