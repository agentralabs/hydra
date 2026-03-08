//! JsCompiler — Prime AST → JavaScript code.

use crate::prime::ast::*;

/// Compiles Prime AST to JavaScript
pub struct JsCompiler {
    indent: usize,
}

impl JsCompiler {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        Ok(self.emit(node))
    }

    fn emit(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!("class {} {{\n", name);
                self.indent += 1;
                s.push_str(&format!(
                    "{}constructor({}) {{\n",
                    self.pad(),
                    fields
                        .iter()
                        .map(|f| f.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                self.indent += 1;
                for f in fields {
                    s.push_str(&format!("{}this.{} = {};\n", self.pad(), f.name, f.name));
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}\n", self.pad()));
                self.indent -= 1;
                s.push_str("}\n");
                s
            }
            PrimeNode::Create { entity, data } => {
                let args: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.value(v)))
                    .collect();
                format!(
                    "{}await db.{}.create({{ {} }});\n",
                    self.pad(),
                    Self::camel(entity),
                    args.join(", ")
                )
            }
            PrimeNode::Read {
                entity,
                filter,
                fields,
            } => {
                let fields_str = if fields.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", {{ select: [{}] }}",
                        fields
                            .iter()
                            .map(|f| format!("\"{}\"", f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                if let Some(f) = filter {
                    format!(
                        "{}await db.{}.findMany({{ where: {{ {}: {} }} }}{});\n",
                        self.pad(),
                        Self::camel(entity),
                        f.field,
                        self.value(&f.value),
                        fields_str
                    )
                } else {
                    format!(
                        "{}await db.{}.findMany({{ }}{});\n",
                        self.pad(),
                        Self::camel(entity),
                        fields_str
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
                    .map(|(k, v)| format!("{}: {}", k, self.value(v)))
                    .collect();
                format!(
                    "{}await db.{}.updateMany({{ where: {{ {}: {} }}, data: {{ {} }} }});\n",
                    self.pad(),
                    Self::camel(entity),
                    filter.field,
                    self.value(&filter.value),
                    data_str.join(", ")
                )
            }
            PrimeNode::Delete { entity, filter } => {
                format!(
                    "{}await db.{}.deleteMany({{ where: {{ {}: {} }} }});\n",
                    self.pad(),
                    Self::camel(entity),
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
                    "{}if ({} {} {}) {{\n",
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
                    "{}for (const {} of {}) {{\n",
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
                let mut s = format!(
                    "{}app.{}('{}', async (req, res) => {{\n",
                    self.pad(),
                    m,
                    path
                );
                self.indent += 1;
                s.push_str(&self.emit(handler));
                self.indent -= 1;
                s.push_str(&format!("{}}});\n", self.pad()));
                s
            }
            PrimeNode::Api { name, endpoints } => {
                let mut s = format!("// API: {}\nconst app = express();\n\n", name);
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
                    "{}await {}.{}({});\n",
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
                format!("{}const {} = {};\n", self.pad(), name, self.value(value))
            }
            PrimeNode::Store { key, value } => {
                format!(
                    "{}store.set('{}', {});\n",
                    self.pad(),
                    key,
                    self.emit(value)
                )
            }
            PrimeNode::Raw { content } => {
                format!("{}{}\n", self.pad(), content)
            }
        }
    }

    fn value(&self, v: &PrimeValue) -> String {
        match v {
            PrimeValue::Null => "null".into(),
            PrimeValue::Bool(b) => b.to_string(),
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
                "{{ {} }}",
                m.iter()
                    .map(|(k, v)| format!("{}: {}", k, self.value(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn pad(&self) -> String {
        "  ".repeat(self.indent)
    }

    fn op(op: &FilterOp) -> &'static str {
        match op {
            FilterOp::Eq => "===",
            FilterOp::Ne => "!==",
            FilterOp::Gt => ">",
            FilterOp::Gte => ">=",
            FilterOp::Lt => "<",
            FilterOp::Lte => "<=",
            FilterOp::Like => ".includes",
            FilterOp::In => "in",
        }
    }

    fn camel(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().to_string() + c.as_str(),
        }
    }
}

impl Default for JsCompiler {
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
            fields: vec![Field {
                name: "name".into(),
                type_: PrimeType::String,
                constraints: vec![],
                optional: false,
            }],
        };
        let code = JsCompiler::new().compile(&node).unwrap();
        assert!(code.contains("class User"));
        assert!(code.contains("this.name = name"));
    }

    #[test]
    fn test_compile_api() {
        let node = PrimeNode::Api {
            name: "TestAPI".into(),
            endpoints: vec![PrimeNode::Endpoint {
                method: HttpMethod::Get,
                path: "/users".into(),
                params: vec![],
                handler: Box::new(PrimeNode::Return {
                    value: PrimeValue::String("ok".into()),
                }),
            }],
        };
        let code = JsCompiler::new().compile(&node).unwrap();
        assert!(code.contains("app.get('/users'"));
        assert!(code.contains("express()"));
    }
}
