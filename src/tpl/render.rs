use crate::models::db_type::DatabaseType;
use crate::rdbc::value::Value;
use crate::tpl::ast::{AstNode, Context, RenderBuffer};
use crate::tpl::cache::TEMPLATE_CACHE;

fn eval_atom(expr: &str, ctx: &Context) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return false;
    }

    let (key, val_str, is_eq) = if let Some((k, v)) = expr.split_once("!=") {
        (k.trim(), v.trim(), false)
    } else if let Some((k, v)) = expr.split_once("==") {
        (k.trim(), v.trim(), true)
    } else {
        let val = ctx.lookup(expr);
        return !matches!(val, Value::Null | Value::Bool(false));
    };

    let left = ctx.lookup(key);

    let equal = if val_str == "null" {
        matches!(left, Value::Null)
    } else if val_str == "true" {
        matches!(left, Value::Bool(true))
    } else if val_str == "false" {
        matches!(left, Value::Bool(false))
    } else if (val_str.starts_with('\'') && val_str.ends_with('\''))
        || (val_str.starts_with('"') && val_str.ends_with('"'))
    {
        match left {
            Value::Str(s) => s == &val_str[1..val_str.len() - 1],
            _ => false,
        }
    } else {
        // Try parsing as number if it looks like one
        let first = val_str.as_bytes()[0];
        if first.is_ascii_digit() || first == b'-' {
            if let Ok(n) = val_str.parse::<i64>() {
                match left {
                    Value::I64(v) => *v == n,
                    Value::F64(v) => *v == n as f64,
                    Value::I32(v) => *v as i64 == n,
                    Value::I16(v) => *v as i64 == n,
                    Value::U8(v) => *v as i64 == n,
                    _ => false,
                }
            } else if let Ok(n) = val_str.parse::<f64>() {
                match left {
                    Value::F64(v) => *v == n,
                    Value::I64(v) => *v as f64 == n,
                    Value::I32(v) => *v as f64 == n,
                    Value::I16(v) => *v as f64 == n,
                    Value::U8(v) => *v as f64 == n,
                    _ => false,
                }
            } else {
                // Fallback to lookup (e.g. if parsing failed but started with digit/hyphen, unlikely for valid vars but safe)
                let right = ctx.lookup(val_str);
                left == right
            }
        } else {
            let right = ctx.lookup(val_str);
            left == right
        }
    };

    if is_eq { equal } else { !equal }
}

pub fn eval_expr(expr: &str, ctx: &Context) -> bool {
    for or_part in expr.split(" or ") {
        let mut and_satisfied = true;
        for atom in or_part.split(" and ") {
            if !eval_atom(atom, ctx) {
                and_satisfied = false;
                break;
            }
        }
        if and_satisfied {
            return true;
        }
    }
    false
}

pub(crate) fn render(nodes: &[AstNode], ctx: &mut Context, buf: &mut RenderBuffer) {
    for node in nodes {
        match node {
            AstNode::Text(t) => buf.sql.push_str(t),
            AstNode::Var(name) => {
                let v = ctx.lookup(name);
                buf.params.push((name.clone(), v.clone()));
                match buf.db_type {
                    DatabaseType::MySql | DatabaseType::Sqlite | DatabaseType::Mssql => {
                        buf.sql.push('?')
                    }
                    DatabaseType::Postgres => {
                        buf.param_count += 1;
                        buf.sql.push('$');
                        buf.sql.push_str(&buf.param_count.to_string());
                    }
                    DatabaseType::Oracle => {
                        buf.sql.push(':');
                        buf.sql.push_str(name);
                    }
                }
            }
            AstNode::Include { refid } => {
                if let Some(cached) = TEMPLATE_CACHE.get(refid) {
                    render(&cached.ast, ctx, buf);
                }
            }
            AstNode::If { test, body } => {
                if eval_expr(test, ctx) {
                    render(body, ctx, buf);
                }
            }
            AstNode::For {
                item,
                collection,
                open,
                sep,
                close,
                body,
            } => {
                let arr = match ctx.lookup(collection) {
                    Value::List(v) => v,
                    _ => continue,
                };
                if arr.is_empty() {
                    continue;
                }

                buf.sql.push_str(open);
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        buf.sql.push_str(sep);
                    }

                    ctx.push(item, v);
                    render(body, ctx, buf);
                    ctx.pop();
                }
                buf.sql.push_str(close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_eval_atom_literals() {
        let root = Value::Map(HashMap::new());
        let ctx = Context::new(&root);

        // Truthy check
        // We need to mock lookup. Since context is empty, lookup returns Null.
        // Null is falsey.
        assert_eq!(eval_atom("var", &ctx), false);

        // We need a context with values.
        let mut map = HashMap::new();
        map.insert("a".to_string(), Value::I64(10));
        map.insert("b".to_string(), Value::Str("hello".to_string()));
        map.insert("c".to_string(), Value::Bool(true));
        let root = Value::Map(map);
        let ctx = Context::new(&root);

        assert!(eval_atom("a == 10", &ctx));
        assert!(eval_atom("a != 5", &ctx));
        assert!(eval_atom("b == 'hello'", &ctx));
        assert!(eval_atom("b != 'world'", &ctx));
        assert!(eval_atom("c", &ctx));
        assert!(eval_atom("c == true", &ctx));
    }

    #[test]
    fn test_eval_expr() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), Value::I64(1));
        map.insert("y".to_string(), Value::I64(2));
        let root = Value::Map(map);
        let ctx = Context::new(&root);

        assert!(eval_expr("x == 1 and y == 2", &ctx));
        assert!(eval_expr("x == 1 or y == 3", &ctx));
        assert!(!eval_expr("x == 2 or y == 3", &ctx));
    }
}
