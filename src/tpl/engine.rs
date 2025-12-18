use crate::models::db_type::DatabaseType;
use crate::rdbc::serializer::to_value;
use crate::rdbc::value::Value;
use crate::tpl::ast::{Context, RenderBuffer};
use crate::tpl::{cache, render};

/// 渲染模板，返回 SQL 和参数
pub fn render_template<T: serde::Serialize>(
    template_name: &str,
    template_content: &str,
    param: &T,
    db_type: DatabaseType,
) -> (String, Vec<(String, Value)>) {
    // 获取 AST（缓存）
    let ast = cache::get_ast(template_name, template_content);

    // 序列化参数为 Value
    let value = to_value(param);

    // 创建渲染上下文
    let mut buf = RenderBuffer {
        sql: String::with_capacity(template_content.len()),
        params: Vec::with_capacity(10),
        db_type,
        param_count: 0,
    };

    let mut ctx = Context::new(&value);
    render::render(&ast, &mut ctx, &mut buf);

    (buf.sql, buf.params)
}

/// 卸载模板缓存
pub fn remove_template(template_name: &str) {
    cache::TEMPLATE_CACHE.remove(template_name);
}
