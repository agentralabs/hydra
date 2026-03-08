//! PyCompiler — Prime AST → Python code.

use crate::prime::ast::*;

/// Compiles Prime AST to Python
pub struct PyCompiler {
    indent: usize,
}

impl PyCompiler {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        Ok(self.emit(node))
    }

    fn emit(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!("@dataclass\nclass {}:\n", name);
                self.indent += 1;
                if fields.is_empty() {
                    s.push_str(&format!("{}pass\n", self.pad()));
                } else {
                    for f in fields {
                        let opt = if f.optional { " = None" } else { "" };
                        s.push_str(&format!(
                            "{}{}: {}{}\n",
                            self.pad(),
                            f.name,
                            Self::type_hint(&f.type_),
                            opt
                        ));
                    }
                }
                self.indent -= 1;
                s
            }
            PrimeNode::Create { entity, data } => {
                let args: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.value(v)))
                    .collect();
                format!(
                    "{}await db.{}.create({})\n",
                    self.pad(),
                    Self::snake(entity),
                    args.join(", ")
                )
            }
            PrimeNode::Read { entity, filter, .. } => {
                if let Some(f) = filter {
                    format!(
                        "{}await db.{}.find_many({}={})\n",
                        self.pad(),
                        Self::snake(entity),
                        f.field,
                        self.value(&f.value)
                    )
                } else {
                    format!(
                        "{}await db.{}.find_many()\n",
                        self.pad(),
                        Self::snake(entity)
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
                    .map(|(k, v)| format!("{}={}", k, self.value(v)))
                    .collect();
                format!(
                    "{}await db.{}.update_many(where_=dict({}={}), data=dict({}))\n",
                    self.pad(),
                    Self::snake(entity),
                    filter.field,
                    self.value(&filter.value),
                    data_str.join(", ")
                )
            }
            PrimeNode::Delete { entity, filter } => {
                format!(
                    "{}await db.{}.delete_many({}={})\n",
                    self.pad(),
                    Self::snake(entity),
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
                    "{}if {} {} {}:\n",
                    self.pad(),
                    self.value(&condition.left),
                    Self::op(&condition.op),
                    self.value(&condition.right)
                );
                self.indent += 1;
                s.push_str(&self.emit(then));
                self.indent -= 1;
                if let Some(e) = else_branch {
                    s.push_str(&format!("{}else:\n", self.pad()));
                    self.indent += 1;
                    s.push_str(&self.emit(e));
                    self.indent -= 1;
                }
                s
            }
            PrimeNode::Loop {
                variable,
                collection,
                body,
            } => {
                let mut s = format!(
                    "{}for {} in {}:\n",
                    self.pad(),
                    variable,
                    self.value(collection)
                );
                self.indent += 1;
                s.push_str(&self.emit(body));
                self.indent -= 1;
                s
            }
            PrimeNode::Endpoint {
                method,
                path,
                handler,
                ..
            } => {
                let m = method.as_str().to_lowercase();
                let mut s = format!("{}@app.{}(\"{}\")\n", self.pad(), m, path);
                s.push_str(&format!(
                    "{}async def handle_{}():\n",
                    self.pad(),
                    Self::snake_path(path)
                ));
                self.indent += 1;
                s.push_str(&self.emit(handler));
                self.indent -= 1;
                s.push('\n');
                s
            }
            PrimeNode::Api { name, endpoints } => {
                let mut s = format!("# API: {}\napp = FastAPI()\n\n", name);
                for ep in endpoints {
                    s.push_str(&self.emit(ep));
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
                    "{}await {}.{}({})\n",
                    self.pad(),
                    target,
                    method,
                    args_str.join(", ")
                )
            }
            PrimeNode::Return { value } => {
                format!("{}return {}\n", self.pad(), self.value(value))
            }
            PrimeNode::Assign { name, value } => {
                format!("{}{} = {}\n", self.pad(), name, self.value(value))
            }
            PrimeNode::Store { key, value } => {
                format!(
                    "{}store[\"{}\"] = {}\n",
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
            PrimeValue::Bool(b) => {
                if *b {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{}", f),
            PrimeValue::String(s) => format!("\"{}\"", s),
            PrimeValue::Variable(v) => v.clone(),
            PrimeValue::Array(arr) => format!(
                "[{}]",
                arr.iter()
                    .map(|v| self.value(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PrimeValue::Map(m) => format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.value(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn pad(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn type_hint(t: &PrimeType) -> &'static str {
        match t {
            PrimeType::String => "str",
            PrimeType::Integer => "int",
            PrimeType::Float => "float",
            PrimeType::Boolean => "bool",
            PrimeType::DateTime => "datetime",
            PrimeType::Uuid => "UUID",
            PrimeType::Map => "dict",
            PrimeType::Any => "Any",
            PrimeType::Array(_) => "list",
            PrimeType::Entity(_) => "object",
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
            FilterOp::Like => "in",
            FilterOp::In => "in",
        }
    }

    fn snake(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        }
        result
    }

    fn snake_path(path: &str) -> String {
        path.trim_start_matches('/')
            .replace('/', "_")
            .replace('-', "_")
    }
}

impl Default for PyCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_entity() {
        let node = PrimeNode::Entity {
            name: "User".into(),
            fields: vec![
                Field {
                    name: "name".into(),
                    type_: PrimeType::String,
                    constraints: vec![],
                    optional: false,
                },
                Field {
                    name: "email".into(),
                    type_: PrimeType::String,
                    constraints: vec![],
                    optional: true,
                },
            ],
        };
        let code = PyCompiler::new().compile(&node).unwrap();
        assert!(code.contains("@dataclass"));
        assert!(code.contains("class User:"));
        assert!(code.contains("name: str"));
        assert!(code.contains("email: str = None"));
    }

    #[test]
    fn test_compile_conditional() {
        let node = PrimeNode::Conditional {
            condition: Condition {
                left: PrimeValue::Variable("x".into()),
                op: FilterOp::Gt,
                right: PrimeValue::Int(0),
            },
            then: Box::new(PrimeNode::Return {
                value: PrimeValue::Bool(true),
            }),
            else_branch: None,
        };
        let code = PyCompiler::new().compile(&node).unwrap();
        assert!(code.contains("if x > 0:"));
        assert!(code.contains("return True"));
    }
}
