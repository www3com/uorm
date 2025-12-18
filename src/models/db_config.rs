pub struct ConnectionOptions {
    /// 格式：mysql://username:password@host:port/database
    pub url: String,
    pub max_open_conns: u64, // 设置池最大连接数
    pub max_idle_conns: u64, // 设置池最大空闲数
    pub max_lifetime: u64,   // 设置连接最大生命周期
    pub timeout: u64,        // 设置连接池获取连接的超时时间
}

pub struct UormOptions<'a> {
    pub assets: Option<Vec<&'a [u8]>>, // mapper 文件，从二进制文件中读取
    pub assets_path: Option<&'a str>,  // mapper 文件，从文件系统中读取
    pub conn_options: ConnectionOptions,
}

impl<'a> UormOptions<'a> {
    pub fn new(url: String) -> Self {
        UormOptions {
            conn_options: ConnectionOptions {
                url: url.clone(),
                max_open_conns: 10,
                max_idle_conns: 2,
                max_lifetime: 30_60,
                timeout: 10,
            },
            assets: None,
            assets_path: None,
        }
    }
    pub fn max_open_conns(mut self, max_open_conns: u64) -> Self {
        self.conn_options.max_open_conns = max_open_conns;
        self
    }

    pub fn max_idle_conns(mut self, max_idle_conns: u64) -> Self {
        self.conn_options.max_idle_conns = max_idle_conns;
        self
    }
    pub fn max_lifetime(mut self, max_lifetime: u64) -> Self {
        self.conn_options.max_lifetime = max_lifetime;
        self
    }
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.conn_options.timeout = timeout;
        self
    }

    pub fn assets(mut self, assets: Vec<&'a [u8]>) -> Self {
        self.assets = Some(assets);
        self
    }

    pub fn assets_path(mut self, assets_path: &'a str) -> Self {
        self.assets_path = Some(assets_path);
        self
    }
}
