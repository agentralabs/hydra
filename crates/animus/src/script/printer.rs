//! ScriptPrinter — render Prime AST back to Animus Script text.

use crate::prime::ast::*;

/// Renders Prime AST to Animus Script
pub struct ScriptPrinter {
    indent: usize,
}

impl ScriptPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    /// Render a Prime AST node to Animus Script text
    pub fn print(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!("{}entity {} {{\n", self.indent_str(), name);
                self.indent += 1;
                for (i, field) in fields.iter().enumerate() {
                    s.push_str(&format!(
                        "{}{}: {}",
                        self.indent_str(),
                        field.name,
                        Self::type_name(&field.type_)
                    ));
                    if field.optional {
                        s.push_str(" optional");
                    }
                    if i < fields.len() - 1 {
                        s.push(',');
                    }
                    s.push('\n');
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                s
            }
            PrimeNode::Create { entity, data } => {
                let mut s = format!("{}create {} {{\n", self.indent_str(), entity);
                self.indent += 1;
                let entries: Vec<_> = data.iter().collect();
                for (i, (k, v)) in entries.iter().enumerate() {
                    s.push_str(&format!(
                        "{}{}: {}",
                        self.indent_str(),
                        k,
                        Self::value_str(v)
                    ));
                    if i < entries.len() - 1 {
                        s.push(',');
                    }
                    s.push('\n');
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                s
            }
            PrimeNode::Read { entity, filter, .. } => {
                if let Some(f) = filter {
                    format!(
                        "{}read {} {{ {} {} {} }}",
                        self.indent_str(),
                        entity,
                        f.field,
                        Self::op_str(&f.op),
                        Self::value_str(&f.value)
                    )
                } else {
                    format!("{}read {}", self.indent_str(), entity)
                }
            }
            PrimeNode::Update {
                entity,
                filter,
                data,
            } => {
                let data_str: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::value_str(v)))
                    .collect();
                format!(
                    "{}update {} {{ {} {} {}, {} }}",
                    self.indent_str(),
                    entity,
                    filter.field,
                    Self::op_str(&filter.op),
                    Self::value_str(&filter.value),
                    data_str.join(", ")
                )
            }
            PrimeNode::Delete { entity, filter } => {
                format!(
                    "{}delete {} {{ {} {} {} }}",
                    self.indent_str(),
                    entity,
                    filter.field,
                    Self::op_str(&filter.op),
                    Self::value_str(&filter.value)
                )
            }
            PrimeNode::Sequence(nodes) => nodes
                .iter()
                .map(|n| self.print(n))
                .collect::<Vec<_>>()
                .join("\n"),
            PrimeNode::Conditional {
                condition,
                then,
                else_branch,
            } => {
                let mut s = format!(
                    "{}if {} {} {} {{\n",
                    self.indent_str(),
                    Self::value_str(&condition.left),
                    Self::op_str(&condition.op),
                    Self::value_str(&condition.right)
                );
                self.indent += 1;
                s.push_str(&self.print(then));
                s.push('\n');
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                if let Some(e) = else_branch {
                    s.push_str(" else {\n");
                    self.indent += 1;
                    s.push_str(&self.print(e));
                    s.push('\n');
                    self.indent -= 1;
                    s.push_str(&format!("{}}}", self.indent_str()));
                }
                s
            }
            PrimeNode::Loop {
                variable,
                collection,
                body,
            } => {
                let mut s = format!(
                    "{}for {} in {} {{\n",
                    self.indent_str(),
                    variable,
                    Self::value_str(collection)
                );
                self.indent += 1;
                s.push_str(&self.print(body));
                s.push('\n');
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                s
            }
            PrimeNode::Endpoint {
                method,
                path,
                handler,
                ..
            } => {
                let mut s = format!(
                    "{}endpoint {} \"{}\" -> {{\n",
                    self.indent_str(),
                    method.as_str(),
                    path
                );
                self.indent += 1;
                s.push_str(&self.print(handler));
                s.push('\n');
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                s
            }
            PrimeNode::Api { name, endpoints } => {
                let mut s = format!("{}api {} {{\n", self.indent_str(), name);
                self.indent += 1;
                for ep in endpoints {
                    s.push_str(&self.print(ep));
                    s.push('\n');
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.indent_str()));
                s
            }
            PrimeNode::Call {
                target,
                method,
                args,
            } => {
                let args_str: Vec<String> = args.iter().map(Self::value_str).collect();
                format!(
                    "{}call {}.{}({})",
                    self.indent_str(),
                    target,
                    method,
                    args_str.join(", ")
                )
            }
            PrimeNode::Return { value } => {
                format!("{}return {}", self.indent_str(), Self::value_str(value))
            }
            PrimeNode::Assign { name, value } => {
                format!(
                    "{}let {} = {}",
                    self.indent_str(),
                    name,
                    Self::value_str(value)
                )
            }
            PrimeNode::Store { key, value } => {
                format!("{}store {} = {}", self.indent_str(), key, self.print(value))
            }
            PrimeNode::Raw { content } => {
                format!("{}// raw: {}", self.indent_str(), content)
            }
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn type_name(t: &PrimeType) -> &str {
        match t {
            PrimeType::String => "string",
            PrimeType::Integer => "int",
            PrimeType::Float => "float",
            PrimeType::Boolean => "bool",
            PrimeType::DateTime => "datetime",
            PrimeType::Uuid => "uuid",
            PrimeType::Map => "map",
            PrimeType::Any => "any",
            PrimeType::Array(_) => "array",
            PrimeType::Entity(_) => "entity",
        }
    }

    fn value_str(v: &PrimeValue) -> String {
        match v {
            PrimeValue::Null => "null".into(),
            PrimeValue::Bool(b) => b.to_string(),
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{:.1}", f),
            PrimeValue::String(s) => format!("\"{}\"", s),
            PrimeValue::Variable(v) => v.clone(),
            PrimeValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::value_str).collect();
                format!("[{}]", items.join(", "))
            }
            PrimeValue::Map(m) => {
                let items: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::value_str(v)))
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
        }
    }

    fn op_str(op: &FilterOp) -> &'static str {
        match op {
            FilterOp::Eq => "==",
            FilterOp::Ne => "!=",
            FilterOp::Gt => ">",
            FilterOp::Gte => ">=",
            FilterOp::Lt => "<",
            FilterOp::Lte => "<=",
            FilterOp::Like => "~=",
            FilterOp::In => "in",
        }
    }
}

impl Default for ScriptPrinter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_entity() {
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
                    name: "age".into(),
                    type_: PrimeType::Integer,
                    constraints: vec![],
                    optional: false,
                },
            ],
        };
        let output = ScriptPrinter::new().print(&node);
        assert!(output.contains("entity User"));
        assert!(output.contains("name: string"));
        assert!(output.contains("age: int"));
    }

    #[test]
    fn test_print_return() {
        let node = PrimeNode::Return {
            value: PrimeValue::String("hello".into()),
        };
        let output = ScriptPrinter::new().print(&node);
        assert_eq!(output, "return \"hello\"");
    }

    #[test]
    fn test_print_call() {
        let node = PrimeNode::Call {
            target: "db".into(),
            method: "query".into(),
            args: vec![PrimeValue::String("SELECT *".into())],
        };
        let output = ScriptPrinter::new().print(&node);
        assert_eq!(output, "call db.query(\"SELECT *\")");
    }
}
