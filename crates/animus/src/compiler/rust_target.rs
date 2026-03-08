//! RustCompiler — Prime AST → Rust code.

use crate::prime::ast::*;

/// Compiles Prime AST to Rust
pub struct RustCompiler {
    indent: usize,
}

impl RustCompiler {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        Ok(self.emit(node))
    }

    fn emit(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!(
                    "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n",
                    name
                );
                self.indent += 1;
                for f in fields {
                    let ty = if f.optional {
                        format!("Option<{}>", Self::rust_type(&f.type_))
                    } else {
                        Self::rust_type(&f.type_).to_string()
                    };
                    s.push_str(&format!("{}pub {}: {},\n", self.pad(), f.name, ty));
                }
                self.indent -= 1;
                s.push_str("}\n");
                s
            }
            PrimeNode::Create { entity, data } => {
                let args: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{}{}: {}", self.pad(), k, self.value(v)))
                    .collect();
                format!(
                    "{}let record = {} {{\n{}\n{}}};\n{}db.insert(&record).await?;\n",
                    self.pad(),
                    entity,
                    args.join(",\n"),
                    self.pad(),
                    self.pad()
                )
            }
            PrimeNode::Read { entity, filter, .. } => {
                if let Some(f) = filter {
                    format!(
                        "{}let results = db.query::<{}>().filter(\"{}\", {}).await?;\n",
                        self.pad(),
                        entity,
                        f.field,
                        self.value(&f.value)
                    )
                } else {
                    format!(
                        "{}let results = db.query::<{}>().all().await?;\n",
                        self.pad(),
                        entity
                    )
                }
            }
            PrimeNode::Update {
                entity,
                filter,
                data,
            } => {
                let data_str: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!(".set(\"{}\", {})", k, self.value(v)))
                    .collect();
                format!(
                    "{}db.update::<{}>().filter(\"{}\", {}){}.await?;\n",
                    self.pad(),
                    entity,
                    filter.field,
                    self.value(&filter.value),
                    data_str.join("")
                )
            }
            PrimeNode::Delete { entity, filter } => {
                format!(
                    "{}db.delete::<{}>().filter(\"{}\", {}).await?;\n",
                    self.pad(),
                    entity,
                    filter.field,
                    self.value(&filter.value)
                )
            }
            PrimeNode::Sequence(nodes) => nodes
                .iter()
                .map(|n| self.emit(n))
                .collect::<Vec<_>>()
                .join(""),
            PrimeNode::Conditional {
                condition,
                then,
                else_branch,
            } => {
                let mut s = format!(
                    "{}if {} {} {} {{\n",
                    self.pad(),
                    self.value(&condition.left),
                    Self::op(&condition.op),
                    self.value(&condition.right)
                );
                self.indent += 1;
                s.push_str(&self.emit(then));
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.pad()));
                if let Some(e) = else_branch {
                    s.push_str(" else {\n");
                    self.indent += 1;
                    s.push_str(&self.emit(e));
                    self.indent -= 1;
                    s.push_str(&format!("{}}}", self.pad()));
                }
                s.push('\n');
                s
            }
            PrimeNode::Loop {
                variable,
                collection,
                body,
            } => {
                let mut s = format!(
                    "{}for {} in {} {{\n",
                    self.pad(),
                    variable,
                    self.value(collection)
                );
                self.indent += 1;
                s.push_str(&self.emit(body));
                self.indent -= 1;
                s.push_str(&format!("{}}}\n", self.pad()));
                s
            }
            PrimeNode::Endpoint {
                method,
                path,
                handler,
                ..
            } => {
                let m = method.as_str().to_lowercase();
                let fn_name = format!("handle_{}", path.trim_start_matches('/').replace('/', "_"));
                let mut s = format!(
                    "{}async fn {}() -> impl IntoResponse {{\n",
                    self.pad(),
                    fn_name
                );
                self.indent += 1;
                s.push_str(&self.emit(handler));
                self.indent -= 1;
                s.push_str(&format!("{}}}\n\n", self.pad()));
                s.push_str(&format!(
                    "// Route: .route(\"{}\", {}({}));\n",
                    path, m, fn_name
                ));
                s
            }
            PrimeNode::Api { name, endpoints } => {
                let mut s = format!("// API: {}\n", name);
                s.push_str("let app = Router::new()\n");
                for ep in endpoints {
                    if let PrimeNode::Endpoint { method, path, .. } = ep {
                        let m = method.as_str().to_lowercase();
                        let fn_name =
                            format!("handle_{}", path.trim_start_matches('/').replace('/', "_"));
                        s.push_str(&format!("    .route(\"{}\", {}({}))\n", path, m, fn_name));
                    }
                }
                s.push_str(";\n\n");
                for ep in endpoints {
                    s.push_str(&self.emit(ep));
                    s.push('\n');
                }
                s
            }
            PrimeNode::Call {
                target,
                method,
                args,
            } => {
                let args_str: Vec<String> = args.iter().map(|a| self.value(a)).collect();
                format!(
                    "{}{}.{}({}).await?;\n",
                    self.pad(),
                    target,
                    method,
                    args_str.join(", ")
                )
            }
            PrimeNode::Return { value } => {
                format!("{}return {};\n", self.pad(), self.value(value))
            }
            PrimeNode::Assign { name, value } => {
                format!("{}let {} = {};\n", self.pad(), name, self.value(value))
            }
            PrimeNode::Store { key, value } => {
                format!(
                    "{}store.insert(\"{}\".to_string(), {});\n",
                    self.pad(),
                    key,
                    self.emit(value).trim()
                )
            }
            PrimeNode::Raw { content } => {
                format!("{}{}\n", self.pad(), content)
            }
        }
    }

    fn value(&self, v: &PrimeValue) -> String {
        match v {
            PrimeValue::Null => "None".into(),
            PrimeValue::Bool(b) => b.to_string(),
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{}f64", f),
            PrimeValue::String(s) => format!("\"{}\".to_string()", s),
            PrimeValue::Variable(v) => v.clone(),
            PrimeValue::Array(arr) => format!(
                "vec![{}]",
                arr.iter()
                    .map(|v| self.value(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PrimeValue::Map(m) => {
                let items: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("(\"{}\".to_string(), {})", k, self.value(v)))
                    .collect();
                format!("HashMap::from([{}])", items.join(", "))
            }
        }
    }

    fn pad(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn rust_type(t: &PrimeType) -> &'static str {
        match t {
            PrimeType::String => "String",
            PrimeType::Integer => "i64",
            PrimeType::Float => "f64",
            PrimeType::Boolean => "bool",
            PrimeType::DateTime => "chrono::DateTime<chrono::Utc>",
            PrimeType::Uuid => "uuid::Uuid",
            PrimeType::Map => "HashMap<String, serde_json::Value>",
            PrimeType::Any => "serde_json::Value",
            PrimeType::Array(_) => "Vec<serde_json::Value>",
            PrimeType::Entity(_) => "Box<dyn std::any::Any>",
        }
    }

    fn op(op: &FilterOp) -> &'static str {
        match op {
            FilterOp::Eq => "==",
            FilterOp::Ne => "!=",
            FilterOp::Gt => ">",
            FilterOp::Gte => ">=",
            FilterOp::Lt => "<",
            FilterOp::Lte => "<=",
            FilterOp::Like => ".contains",
            FilterOp::In => "in",
        }
    }
}

impl Default for RustCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_struct() {
        let node = PrimeNode::Entity {
            name: "User".into(),
            fields: vec![
                Field {
                    name: "id".into(),
                    type_: PrimeType::Uuid,
                    constraints: vec![],
                    optional: false,
                },
                Field {
                    name: "name".into(),
                    type_: PrimeType::String,
                    constraints: vec![],
                    optional: false,
                },
                Field {
                    name: "bio".into(),
                    type_: PrimeType::String,
                    constraints: vec![],
                    optional: true,
                },
            ],
        };
        let code = RustCompiler::new().compile(&node).unwrap();
        assert!(code.contains("pub struct User"));
        assert!(code.contains("pub id: uuid::Uuid"));
        assert!(code.contains("pub bio: Option<String>"));
    }

    #[test]
    fn test_compile_loop() {
        let node = PrimeNode::Loop {
            variable: "item".into(),
            collection: PrimeValue::Variable("items".into()),
            body: Box::new(PrimeNode::Return {
                value: PrimeValue::Variable("item".into()),
            }),
        };
        let code = RustCompiler::new().compile(&node).unwrap();
        assert!(code.contains("for item in items"));
    }
}
