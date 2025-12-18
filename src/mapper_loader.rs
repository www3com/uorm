use anyhow::{Context, Result};
use dashmap::DashMap;
use quick_xml::de;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SqlMapper {
    pub content: Option<String>,
    pub use_generated_keys: bool,
    pub key_column: Option<String>,
}

pub type SqlMapperStore = DashMap<String, DashMap<String, SqlMapper>>;

static SQL_MAPPERS: OnceLock<SqlMapperStore> = OnceLock::new();

pub trait AssetProvider {
    fn list(&self) -> Vec<&[u8]>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mapper {
    #[serde(rename = "@namespace")]
    namespace: String,
    #[serde(rename = "$value")]
    nodes: Vec<SqlNode>,
}

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

#[derive(Debug, Deserialize)]
pub struct SqlItem {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@useGeneratedKeys")]
    pub use_generated_keys: Option<String>,
    #[serde(rename = "@keyColumn")]
    pub key_column: Option<String>,
    #[serde(rename = "$text")]
    pub content: Option<String>,
}

impl From<&SqlItem> for SqlMapper {
    fn from(item: &SqlItem) -> Self {
        let use_generated_keys = item
            .use_generated_keys
            .as_ref()
            .map(|s| match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" => true,
                _ => false,
            })
            .unwrap_or(false);
        Self {
            content: item.content.clone(),
            use_generated_keys,
            key_column: item.key_column.clone(),
        }
    }
}

pub fn load(assets: &[&[u8]]) -> Result<()> {
    for data in assets {
        let content = std::str::from_utf8(data).context("Asset content is not valid UTF-8")?;
        process_mapper_data(content, "memory")?;
    }
    Ok(())
}

pub fn find_mapper(sql_id: &str) -> Option<SqlMapper> {
    let (namespace, id) = match sql_id.rfind('.') {
        Some(pos) => (&sql_id[..pos], &sql_id[pos + 1..]),
        None => return None,
    };

    let store = SQL_MAPPERS.get()?;
    let ns_map = store.get(namespace)?;
    ns_map.get(id).map(|v| v.clone())
}

/// 递归读取指定目录及其子目录下的所有 XML 文件，并解析。
pub fn load_from_path(dir_path: &Path) -> Result<()> {
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "xml") {
            process_mapper_file(path)?;
        }
    }
    Ok(())
}

fn process_mapper_file(path: &Path) -> Result<()> {
    let xml_content =
        fs::read_to_string(path).with_context(|| format!("读取文件失败: {}", path.display()))?;
    process_mapper_data(&xml_content, &path.display().to_string())
}

fn process_mapper_data(xml_content: &str, source: &str) -> Result<()> {
    let mapper: Mapper =
        de::from_str(xml_content).with_context(|| format!("XML 解析失败: {}", source))?;
    let namespace = mapper.namespace;

    let store = SQL_MAPPERS.get_or_init(|| DashMap::new());

    let ns_map = store
        .entry(namespace.clone())
        .or_insert_with(|| DashMap::new());

    for node in mapper.nodes {
        if let Some(item) = node.into_item() {
            let sql_mapper = SqlMapper::from(&item);

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
