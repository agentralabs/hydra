//! GoCompiler — Prime AST → Go code.

use crate::prime::ast::*;

/// Compiles Prime AST to Go
pub struct GoCompiler {
    indent: usize,
}

impl GoCompiler {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        Ok(self.emit(node))
    }

    fn emit(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!("type {} struct {{\n", name);
                self.indent += 1;
                for f in fields {
                    let tag = format!("`json:\"{}\"`", f.name);
                    let ty = if f.optional {
                        format!("*{}", Self::go_type(&f.type_))
                    } else {
                        Self::go_type(&f.type_).to_string()
                    };
                    s.push_str(&format!(
                        "{}{} {} {}\n",
                        self.pad(),
                        Self::pascal(&f.name),
                        ty,
                        tag
                    ));
                }
                self.indent -= 1;
                s.push_str("}\n");
                s
            }
            PrimeNode::Create { entity, data } => {
                let mut s = format!("{}{} := {}{{", self.pad(), Self::camel(entity), entity);
                let args: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{}: {}", Self::pascal(k), self.value(v)))
                    .collect();
                s.push_str(&args.join(", "));
                s.push_str("}\n");
                s.push_str(&format!(
                    "{}err := db.Create(&{}).Error\n",
                    self.pad(),
                    Self::camel(entity)
                ));
                s
            }
            PrimeNode::Read { entity, filter, .. } => {
                if let Some(f) = filter {
                    format!(
                        "{}var results []{}; db.Where(\"{} = ?\", {}).Find(&results)\n",
                        self.pad(),
                        entity,
                        f.field,
                        self.value(&f.value)
                    )
                } else {
                    format!(
                        "{}var results []{}; db.Find(&results)\n",
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
                    .map(|(k, v)| format!("\"{}\": {}", k, self.value(v)))
                    .collect();
                format!("{}db.Model(&{}{{}}).Where(\"{} = ?\", {}).Updates(map[string]interface{{}}{{{}}})\n",
                    self.pad(), entity, filter.field, self.value(&filter.value), data_str.join(", "))
            }
            PrimeNode::Delete { entity, filter } => {
                format!(
                    "{}db.Where(\"{} = ?\", {}).Delete(&{}{{}})\n",
                    self.pad(),
                    filter.field,
                    self.value(&filter.value),
                    entity
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
                    "{}for _, {} := range {} {{\n",
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
                let fn_name = format!(
                    "Handle{}",
                    Self::pascal(&path.replace('/', "_").trim_start_matches('_').to_string())
                );
                let mut s = format!(
                    "{}func {}(w http.ResponseWriter, r *http.Request) {{\n",
                    self.pad(),
                    fn_name
                );
                self.indent += 1;
                s.push_str(&self.emit(handler));
                self.indent -= 1;
                s.push_str(&format!("{}}}\n\n", self.pad()));
                s.push_str(&format!(
                    "// Route: r.HandleFunc(\"{}\", {}).Methods(\"{}\")\n",
                    path,
                    fn_name,
                    method.as_str()
                ));
                s
            }
            PrimeNode::Api { name, endpoints } => {
                let mut s = format!(
                    "// API: {}\nfunc Setup{}Routes(r *mux.Router) {{\n",
                    name, name
                );
                self.indent += 1;
                for ep in endpoints {
                    if let PrimeNode::Endpoint { method, path, .. } = ep {
                        let fn_name = format!(
                            "Handle{}",
                            Self::pascal(
                                &path.replace('/', "_").trim_start_matches('_').to_string()
                            )
                        );
                        s.push_str(&format!(
                            "{}r.HandleFunc(\"{}\", {}).Methods(\"{}\")\n",
                            self.pad(),
                            path,
                            fn_name,
                            method.as_str()
                        ));
                    }
                }
                self.indent -= 1;
                s.push_str("}\n\n");
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
                    "{}{}.{}({})\n",
                    self.pad(),
                    target,
                    Self::pascal(method),
                    args_str.join(", ")
                )
            }
            PrimeNode::Return { value } => {
                format!("{}return {}\n", self.pad(), self.value(value))
            }
            PrimeNode::Assign { name, value } => {
                format!("{}{} := {}\n", self.pad(), name, self.value(value))
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
            PrimeValue::Null => "nil".into(),
            PrimeValue::Bool(b) => b.to_string(),
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{}", f),
            PrimeValue::String(s) => format!("\"{}\"", s),
            PrimeValue::Variable(v) => v.clone(),
            PrimeValue::Array(arr) => format!(
                "[]interface{{}}{{{}}}",
                arr.iter()
                    .map(|v| self.value(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PrimeValue::Map(m) => {
                let items: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.value(v)))
                    .collect();
                format!("map[string]interface{{}}{{{}}}", items.join(", "))
            }
        }
    }

    fn pad(&self) -> String {
        "\t".repeat(self.indent)
    }

    fn go_type(t: &PrimeType) -> &'static str {
        match t {
            PrimeType::String => "string",
            PrimeType::Integer => "int64",
            PrimeType::Float => "float64",
            PrimeType::Boolean => "bool",
            PrimeType::DateTime => "time.Time",
            PrimeType::Uuid => "uuid.UUID",
            PrimeType::Map => "map[string]interface{}",
            PrimeType::Any => "interface{}",
            PrimeType::Array(_) => "[]interface{}",
            PrimeType::Entity(_) => "interface{}",
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
            FilterOp::Like => "==",
            FilterOp::In => "==",
        }
    }

    fn pascal(s: &str) -> String {
        s.split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            })
            .collect()
    }

    fn camel(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().to_string() + c.as_str(),
        }
    }
}

impl Default for GoCompiler {
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
            fields: vec![Field {
                name: "name".into(),
                type_: PrimeType::String,
                constraints: vec![],
                optional: false,
            }],
        };
        let code = GoCompiler::new().compile(&node).unwrap();
        assert!(code.contains("type User struct"));
        assert!(code.contains("Name string"));
        assert!(code.contains("`json:\"name\"`"));
    }

    #[test]
    fn test_compile_for_range() {
        let node = PrimeNode::Loop {
            variable: "item".into(),
            collection: PrimeValue::Variable("items".into()),
            body: Box::new(PrimeNode::Return {
                value: PrimeValue::Variable("item".into()),
            }),
        };
        let code = GoCompiler::new().compile(&node).unwrap();
        assert!(code.contains("for _, item := range items"));
    }
}
