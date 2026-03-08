//! SqlCompiler — Prime AST → SQL statements.

use crate::prime::ast::*;

/// Compiles Prime AST to SQL
pub struct SqlCompiler;

impl SqlCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        self.emit(node)
    }

    fn emit(&self, node: &PrimeNode) -> Result<String, String> {
        match node {
            PrimeNode::Entity { name, fields } => {
                let mut s = format!("CREATE TABLE {} (\n", Self::snake(name));
                let cols: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let mut col = format!("    {} {}", f.name, Self::sql_type(&f.type_));
                        if !f.optional {
                            col.push_str(" NOT NULL");
                        }
                        for c in &f.constraints {
                            match c {
                                Constraint::PrimaryKey => col.push_str(" PRIMARY KEY"),
                                Constraint::Unique => col.push_str(" UNIQUE"),
                                Constraint::Default(v) => {
                                    col.push_str(&format!(" DEFAULT {}", Self::sql_value(v)))
                                }
                                Constraint::ForeignKey { entity, field } => {
                                    col.push_str(&format!(
                                        " REFERENCES {}({})",
                                        Self::snake(entity),
                                        field
                                    ));
                                }
                                _ => {}
                            }
                        }
                        col
                    })
                    .collect();
                s.push_str(&cols.join(",\n"));
                s.push_str("\n);\n");
                Ok(s)
            }
            PrimeNode::Create { entity, data } => {
                let cols: Vec<&String> = data.keys().collect();
                let vals: Vec<String> = data.values().map(Self::sql_value).collect();
                Ok(format!(
                    "INSERT INTO {} ({}) VALUES ({});\n",
                    Self::snake(entity),
                    cols.iter()
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    vals.join(", ")
                ))
            }
            PrimeNode::Read {
                entity,
                filter,
                fields,
            } => {
                let select = if fields.is_empty() {
                    "*".to_string()
                } else {
                    fields.join(", ")
                };
                let where_clause = filter
                    .as_ref()
                    .map(|f| {
                        format!(
                            " WHERE {} {} {}",
                            f.field,
                            Self::sql_op(&f.op),
                            Self::sql_value(&f.value)
                        )
                    })
                    .unwrap_or_default();
                Ok(format!(
                    "SELECT {} FROM {}{};\n",
                    select,
                    Self::snake(entity),
                    where_clause
                ))
            }
            PrimeNode::Update {
                entity,
                filter,
                data,
            } => {
                let sets: Vec<String> = data
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, Self::sql_value(v)))
                    .collect();
                Ok(format!(
                    "UPDATE {} SET {} WHERE {} {} {};\n",
                    Self::snake(entity),
                    sets.join(", "),
                    filter.field,
                    Self::sql_op(&filter.op),
                    Self::sql_value(&filter.value)
                ))
            }
            PrimeNode::Delete { entity, filter } => Ok(format!(
                "DELETE FROM {} WHERE {} {} {};\n",
                Self::snake(entity),
                filter.field,
                Self::sql_op(&filter.op),
                Self::sql_value(&filter.value)
            )),
            PrimeNode::Sequence(nodes) => {
                let stmts: Result<Vec<String>, String> =
                    nodes.iter().map(|n| self.emit(n)).collect();
                Ok(stmts?.join(""))
            }
            PrimeNode::Conditional {
                condition,
                then,
                else_branch,
            } => {
                let mut s = format!(
                    "CASE WHEN {} {} {} THEN\n",
                    Self::sql_value(&condition.left),
                    Self::sql_op(&condition.op),
                    Self::sql_value(&condition.right)
                );
                s.push_str(&format!("    {}", self.emit(then)?.trim()));
                if let Some(e) = else_branch {
                    s.push_str(&format!("\nELSE\n    {}", self.emit(e)?.trim()));
                }
                s.push_str("\nEND;\n");
                Ok(s)
            }
            _ => Ok(format!("-- unsupported node: {}\n", node.type_name())),
        }
    }

    fn sql_value(v: &PrimeValue) -> String {
        match v {
            PrimeValue::Null => "NULL".into(),
            PrimeValue::Bool(b) => {
                if *b {
                    "TRUE".into()
                } else {
                    "FALSE".into()
                }
            }
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{}", f),
            PrimeValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            PrimeValue::Variable(v) => format!(":{}", v),
            _ => "NULL".into(),
        }
    }

    fn sql_type(t: &PrimeType) -> &'static str {
        match t {
            PrimeType::String => "TEXT",
            PrimeType::Integer => "BIGINT",
            PrimeType::Float => "DOUBLE PRECISION",
            PrimeType::Boolean => "BOOLEAN",
            PrimeType::DateTime => "TIMESTAMP WITH TIME ZONE",
            PrimeType::Uuid => "UUID",
            PrimeType::Map => "JSONB",
            PrimeType::Any => "JSONB",
            PrimeType::Array(_) => "JSONB",
            PrimeType::Entity(_) => "UUID",
        }
    }

    fn sql_op(op: &FilterOp) -> &'static str {
        match op {
            FilterOp::Eq => "=",
            FilterOp::Ne => "!=",
            FilterOp::Gt => ">",
            FilterOp::Gte => ">=",
            FilterOp::Lt => "<",
            FilterOp::Lte => "<=",
            FilterOp::Like => "LIKE",
            FilterOp::In => "IN",
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table() {
        let node = PrimeNode::Entity {
            name: "User".into(),
            fields: vec![
                Field {
                    name: "id".into(),
                    type_: PrimeType::Uuid,
                    constraints: vec![Constraint::PrimaryKey],
                    optional: false,
                },
                Field {
                    name: "name".into(),
                    type_: PrimeType::String,
                    constraints: vec![],
                    optional: false,
                },
                Field {
                    name: "email".into(),
                    type_: PrimeType::String,
                    constraints: vec![Constraint::Unique],
                    optional: false,
                },
            ],
        };
        let sql = SqlCompiler::new().compile(&node).unwrap();
        assert!(sql.contains("CREATE TABLE user"));
        assert!(sql.contains("id UUID NOT NULL PRIMARY KEY"));
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
    }

    #[test]
    fn test_insert() {
        let node = PrimeNode::Create {
            entity: "User".into(),
            data: std::collections::HashMap::from([(
                "name".into(),
                PrimeValue::String("Alice".into()),
            )]),
        };
        let sql = SqlCompiler::new().compile(&node).unwrap();
        assert!(sql.contains("INSERT INTO user"));
        assert!(sql.contains("'Alice'"));
    }

    #[test]
    fn test_select_with_filter() {
        let node = PrimeNode::Read {
            entity: "User".into(),
            filter: Some(Filter {
                field: "id".into(),
                op: FilterOp::Eq,
                value: PrimeValue::Int(1),
            }),
            fields: vec!["name".into(), "email".into()],
        };
        let sql = SqlCompiler::new().compile(&node).unwrap();
        assert!(sql.contains("SELECT name, email FROM user WHERE id = 1"));
    }
}
