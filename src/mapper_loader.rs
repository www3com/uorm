use anyhow::{Context, Result};
use dashmap::DashMap;
use glob::glob;
use quick_xml::de;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// SQL 映射对象，包含 SQL 内容及相关配置
#[derive(Debug, Clone)]
pub struct SqlMapper {
    /// SQL 文本内容
    pub content: Option<String>,
    /// 是否使用数据库自增主键
    pub use_generated_keys: bool,
    /// 主键列名
    pub key_column: Option<String>,
}

/// SQL 映射器存储仓库，使用 DashMap 实现并发安全的存储
/// 结构：Namespace -> (ID -> SqlMapper)
pub type SqlMapperStore = DashMap<String, DashMap<String, SqlMapper>>;

/// 全局单例的 SQL 映射器存储
static SQL_MAPPERS: OnceLock<SqlMapperStore> = OnceLock::new();

/// 资源提供者特征，用于抽象资源加载
pub trait AssetProvider {
    fn list(&self) -> Vec<&[u8]>;
}

/// XML 映射文件根节点结构
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mapper {
    /// 命名空间，用于隔离不同的 Mapper
    #[serde(rename = "@namespace")]
    namespace: String,
    /// SQL 节点列表
    #[serde(rename = "$value")]
    nodes: Vec<SqlNode>,
}

/// SQL 节点枚举，支持多种 SQL 操作类型
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SqlNode {
    Sql(SqlItem),
    Select(SqlItem),
    Insert(SqlItem),
    Update(SqlItem),
    Delete(SqlItem),
    #[serde(other)]
    Unknown,
}

impl SqlNode {
    /// 将节点转换为统一的 SqlItem
    fn into_item(self) -> Option<SqlItem> {
        match self {
            SqlNode::Sql(item)
            | SqlNode::Select(item)
            | SqlNode::Insert(item)
            | SqlNode::Update(item)
            | SqlNode::Delete(item) => Some(item),
            SqlNode::Unknown => None,
        }
    }
}

/// SQL配置项结构，对应 XML 中的具体标签
#[derive(Debug, Deserialize)]
pub struct SqlItem {
    /// SQL 语句唯一标识
    #[serde(rename = "@id")]
    pub id: String,
    /// 是否使用自增主键配置字符串
    #[serde(rename = "@useGeneratedKeys")]
    pub use_generated_keys: Option<String>,
    /// 主键列名配置
    #[serde(rename = "@keyColumn")]
    pub key_column: Option<String>,
    /// SQL 文本内容
    #[serde(rename = "$text")]
    pub content: Option<String>,
}

impl From<&SqlItem> for SqlMapper {
    fn from(item: &SqlItem) -> Self {
        // 解析 use_generated_keys 属性，支持 true/1/yes
        let use_generated_keys = item
            .use_generated_keys
            .as_deref()
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);

        Self {
            content: item.content.clone(),
            use_generated_keys,
            key_column: item.key_column.clone(),
        }
    }
}

/// 加载指定模式（glob pattern）匹配的所有 XML 映射文件
///
/// # 参数
/// * `pattern` - 文件路径匹配模式，例如 "src/resources/**/*.xml"
///
/// # 返回
/// * `Result<()>` - 加载成功返回 Ok(())，否则返回错误
pub fn load(pattern: &str) -> Result<()> {
    let paths = glob(pattern).with_context(|| format!("读取 glob 模式失败: {}", pattern))?;

    for entry in paths {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    process_mapper_file(&path)?;
                }
            }
            Err(e) => anyhow::bail!("读取路径失败: {}", e),
        }
    }
    Ok(())
}

/// 加载内嵌的 mapper 资源（通常用于编译进二进制的资源）
pub fn load_assets(assets: Vec<(&str, &str)>) -> Result<()> {
    for (source, content) in assets {
        process_mapper_data(content, source)?;
    }
    Ok(())
}

/// 根据 SQL ID 查找对应的 Mapper 配置
///
/// # 参数
/// * `sql_id` - 完整的 SQL ID，格式为 "namespace.id"
pub fn find_mapper(sql_id: &str) -> Option<SqlMapper> {
    // 分割 namespace 和 id
    let (namespace, id) = sql_id.rsplit_once('.')?;

    let store = SQL_MAPPERS.get()?;
    let ns_map = store.get(namespace)?;
    ns_map.get(id).map(|v| v.clone())
}

/// 处理单个 Mapper 文件
fn process_mapper_file(path: &Path) -> Result<()> {
    let xml_content =
        fs::read_to_string(path).with_context(|| format!("读取文件失败: {}", path.display()))?;
    process_mapper_data(&xml_content, &path.display().to_string())
}

/// 解析 Mapper XML 内容并存入全局存储
fn process_mapper_data(xml_content: &str, source: &str) -> Result<()> {
    let mapper: Mapper =
        de::from_str(xml_content).with_context(|| format!("XML 解析失败: {}", source))?;
    let namespace = mapper.namespace;

    // 获取或初始化全局存储
    let store = SQL_MAPPERS.get_or_init(DashMap::new);

    // 获取或初始化命名空间存储
    let ns_map = store.entry(namespace.clone()).or_insert_with(DashMap::new);

    for node in mapper.nodes {
        if let Some(item) = node.into_item() {
            let sql_mapper = SqlMapper::from(&item);

            // 检查 ID 是否重复
            if ns_map.insert(item.id.clone(), sql_mapper).is_some() {
                anyhow::bail!(
                    "文件 '{}' 中发现重复的 ID: '{}' (命名空间: '{}')",
                    source,
                    item.id,
                    namespace
                );
            }
        }
    }
    Ok(())
}

/// 清理所有已加载的 mapper（主要用于测试环境重置状态）
pub fn clear_mappers() {
    if let Some(store) = SQL_MAPPERS.get() {
        store.clear();
    }
}
