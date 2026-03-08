//! ShellCompiler — Prime AST → Shell/Bash scripts.

use crate::prime::ast::*;

/// Compiles Prime AST to Shell script
pub struct ShellCompiler {
    indent: usize,
}

impl ShellCompiler {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn compile(&mut self, node: &PrimeNode) -> Result<String, String> {
        let mut s = "#!/bin/bash\nset -euo pipefail\n\n".to_string();
        s.push_str(&self.emit(node));
        Ok(s)
    }

    fn emit(&mut self, node: &PrimeNode) -> String {
        match node {
            PrimeNode::Assign { name, value } => {
                format!("{}{}={}\n", self.pad(), name, self.shell_value(value))
            }
            PrimeNode::Conditional {
                condition,
                then,
                else_branch,
            } => {
                let mut s = format!(
                    "{}if [ {} {} {} ]; then\n",
                    self.pad(),
                    self.shell_value(&condition.left),
                    Self::shell_op(&condition.op),
                    self.shell_value(&condition.right)
                );
                self.indent += 1;
                s.push_str(&self.emit(then));
                self.indent -= 1;
                if let Some(e) = else_branch {
                    s.push_str(&format!("{}else\n", self.pad()));
                    self.indent += 1;
                    s.push_str(&self.emit(e));
                    self.indent -= 1;
                }
                s.push_str(&format!("{}fi\n", self.pad()));
                s
            }
            PrimeNode::Loop {
                variable,
                collection,
                body,
            } => {
                let mut s = format!(
                    "{}for {} in {}; do\n",
                    self.pad(),
                    variable,
                    self.shell_value(collection)
                );
                self.indent += 1;
                s.push_str(&self.emit(body));
                self.indent -= 1;
                s.push_str(&format!("{}done\n", self.pad()));
                s
            }
            PrimeNode::Sequence(nodes) => nodes
                .iter()
                .map(|n| self.emit(n))
                .collect::<Vec<_>>()
                .join(""),
            PrimeNode::Call {
                target,
                method,
                args,
            } => {
                let args_str: Vec<String> = args.iter().map(|a| self.shell_value(a)).collect();
                if args_str.is_empty() {
                    format!("{}{} {}\n", self.pad(), target, method)
                } else {
                    format!(
                        "{}{} {} {}\n",
                        self.pad(),
                        target,
                        method,
                        args_str.join(" ")
                    )
                }
            }
            PrimeNode::Return { value } => {
                format!("{}echo {}\n", self.pad(), self.shell_value(value))
            }
            PrimeNode::Raw { content } => {
                format!("{}{}\n", self.pad(), content)
            }
            _ => format!("{}# unsupported: {}\n", self.pad(), node.type_name()),
        }
    }

    fn shell_value(&self, v: &PrimeValue) -> String {
        match v {
            PrimeValue::Null => "\"\"".into(),
            PrimeValue::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            PrimeValue::Int(i) => i.to_string(),
            PrimeValue::Float(f) => format!("{}", f),
            PrimeValue::String(s) => format!("\"{}\"", s),
            PrimeValue::Variable(v) => format!("\"${{{}}}\"", v),
            PrimeValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.shell_value(v)).collect();
                format!("({})", items.join(" "))
            }
            PrimeValue::Map(_) => "# map unsupported in shell".into(),
        }
    }

    fn pad(&self) -> String {
        "  ".repeat(self.indent)
    }

    fn shell_op(op: &FilterOp) -> &'static str {
        match op {
            FilterOp::Eq => "-eq",
            FilterOp::Ne => "-ne",
            FilterOp::Gt => "-gt",
            FilterOp::Gte => "-ge",
            FilterOp::Lt => "-lt",
            FilterOp::Lte => "-le",
            FilterOp::Like => "=",
            FilterOp::In => "=",
        }
    }
}

impl Default for ShellCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_conditional() {
        let node = PrimeNode::Conditional {
            condition: Condition {
                left: PrimeValue::Variable("count".into()),
                op: FilterOp::Gt,
                right: PrimeValue::Int(0),
            },
            then: Box::new(PrimeNode::Return {
                value: PrimeValue::String("found".into()),
            }),
            else_branch: None,
        };
        let code = ShellCompiler::new().compile(&node).unwrap();
        assert!(code.contains("if ["));
        assert!(code.contains("-gt"));
        assert!(code.contains("echo \"found\""));
    }

    #[test]
    fn test_compile_loop() {
        let node = PrimeNode::Loop {
            variable: "f".into(),
            collection: PrimeValue::Variable("files".into()),
            body: Box::new(PrimeNode::Call {
                target: "echo".into(),
                method: "Processing".into(),
                args: vec![PrimeValue::Variable("f".into())],
            }),
        };
        let code = ShellCompiler::new().compile(&node).unwrap();
        assert!(code.contains("for f in"));
        assert!(code.contains("done"));
    }
}
