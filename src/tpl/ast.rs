use crate::models::db_type::DatabaseType;
use crate::rdbc::value::Value;

#[derive(Debug, Clone)]
pub enum AstNode {
    Text(String),
    Var(String),
    Include { refid: String },
    If { test: String, body: Vec<AstNode> },
    For { item: String, collection: String, open: String, sep: String, close: String, body: Vec<AstNode> },
}

pub struct RenderBuffer {
    pub sql: String,
    pub params: Vec<(String, Value)>,
    pub db_type: DatabaseType,
    pub param_count: usize,
}

pub struct Context<'a> {
    root: &'a Value,
    locals: Vec<(String, &'a Value)>,
}

impl<'a> Context<'a> {
    pub fn new(root: &'a Value) -> Self {
        Self {
            root,
            locals: Vec::new(),
        }
    }

    pub fn push(&mut self, key: &str, value: &'a Value) {
        self.locals.push((key.to_string(), value));
    }

    pub fn pop(&mut self) {
        self.locals.pop();
    }

    pub fn lookup(&self, key: &str) -> &'a Value {
        // 1. Check locals (stack) in reverse order
        for (k, v) in self.locals.iter().rev() {
            if k == key {
                return v;
            }
        }
        
        // 2. Check root
        if let Value::Map(m) = self.root {
            if let Some(v) = m.get(key) {
                return v;
            }
        }

        // 3. Nested property lookup
        if key.contains('.') {
            let parts: Vec<&str> = key.split('.').collect();
            if parts.is_empty() {
                return &Value::Null;
            }

            let mut current: &'a Value = &Value::Null;
            let mut found_start = false;

            let head = parts[0];

            // Try to find head in locals
            for (k, v) in self.locals.iter().rev() {
                if k == head {
                    current = v;
                    found_start = true;
                    break;
                }
            }

            // Try to find head in root
            if !found_start {
                if let Value::Map(m) = self.root {
                    if let Some(v) = m.get(head) {
                        current = v;
                        found_start = true;
                    }
                }
            }

            if found_start {
                for part in &parts[1..] {
                    match current {
                        Value::Map(m) => {
                            match m.get(*part) {
                                Some(v) => current = v,
                                None => return &Value::Null,
                            }
                        },
                        _ => return &Value::Null,
                    }
                }
                return current;
            }
        }
        
        &Value::Null
    }
}
