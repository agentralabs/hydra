//! ScriptParser — parse Animus Script tokens into Prime AST.

use super::lexer::{Token, TokenKind};
use crate::prime::ast::*;
use std::collections::HashMap;

/// Parses Animus Script tokens into Prime AST
pub struct ScriptParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ScriptParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse all tokens into a sequence of Prime nodes
    pub fn parse(&mut self) -> Result<PrimeNode, String> {
        let mut nodes = Vec::new();
        self.skip_newlines();
        while !self.at_end() {
            let node = self.parse_statement()?;
            nodes.push(node);
            self.skip_newlines();
        }
        if nodes.len() == 1 {
            Ok(nodes.remove(0))
        } else {
            Ok(PrimeNode::Sequence(nodes))
        }
    }

    fn parse_statement(&mut self) -> Result<PrimeNode, String> {
        self.skip_newlines();
        let kind = self.peek_kind();
        match kind {
            TokenKind::Entity => self.parse_entity(),
            TokenKind::Create => self.parse_create(),
            TokenKind::Read => self.parse_read(),
            TokenKind::Update => self.parse_update(),
            TokenKind::Delete => self.parse_delete(),
            TokenKind::If => self.parse_conditional(),
            TokenKind::For => self.parse_loop(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Let => self.parse_assign(),
            TokenKind::Api => self.parse_api(),
            TokenKind::Endpoint => self.parse_endpoint(),
            TokenKind::Call => self.parse_call(),
            _ => Err(format!("unexpected token: {:?}", self.peek())),
        }
    }

    fn parse_entity(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Entity)?;
        let name = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let field = self.parse_field()?;
            fields.push(field);
            self.skip_newlines();
            if self.check(TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Entity { name, fields })
    }

    fn parse_field(&mut self) -> Result<Field, String> {
        let name = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::Colon)?;
        let type_name = self.expect(TokenKind::Ident)?.value;
        let type_ = Self::resolve_type(&type_name);
        let optional = if self.check(TokenKind::Ident) && self.peek().value == "optional" {
            self.advance();
            true
        } else {
            false
        };
        Ok(Field {
            name,
            type_,
            constraints: vec![],
            optional,
        })
    }

    fn resolve_type(name: &str) -> PrimeType {
        match name {
            "string" | "String" => PrimeType::String,
            "int" | "integer" | "Integer" => PrimeType::Integer,
            "float" | "Float" | "number" => PrimeType::Float,
            "bool" | "boolean" | "Boolean" => PrimeType::Boolean,
            "datetime" | "DateTime" => PrimeType::DateTime,
            "uuid" | "Uuid" | "UUID" => PrimeType::Uuid,
            "map" | "Map" | "object" => PrimeType::Map,
            _ => PrimeType::Entity(name.to_string()),
        }
    }

    fn parse_create(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Create)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let data = self.parse_data_map()?;
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Create { entity, data })
    }

    fn parse_read(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Read)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        let filter = if self.check(TokenKind::LBrace) {
            self.advance();
            self.skip_newlines();
            let f = self.parse_filter()?;
            self.expect(TokenKind::RBrace)?;
            Some(f)
        } else {
            None
        };
        Ok(PrimeNode::Read {
            entity,
            filter,
            fields: vec![],
        })
    }

    fn parse_update(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Update)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let filter = self.parse_filter()?;
        if self.check(TokenKind::Comma) {
            self.advance();
        }
        self.skip_newlines();
        let data = self.parse_data_map()?;
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Update {
            entity,
            filter,
            data,
        })
    }

    fn parse_delete(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Delete)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let filter = self.parse_filter()?;
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Delete { entity, filter })
    }

    fn parse_filter(&mut self) -> Result<Filter, String> {
        let field = self.expect(TokenKind::Ident)?.value;
        let op = self.parse_filter_op()?;
        let value = self.parse_value()?;
        Ok(Filter { field, op, value })
    }

    fn parse_filter_op(&mut self) -> Result<FilterOp, String> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::EqEq => Ok(FilterOp::Eq),
            TokenKind::NotEq => Ok(FilterOp::Ne),
            TokenKind::Lt => Ok(FilterOp::Lt),
            TokenKind::Gt => Ok(FilterOp::Gt),
            TokenKind::LtEq => Ok(FilterOp::Lte),
            TokenKind::GtEq => Ok(FilterOp::Gte),
            _ => Err(format!("expected filter op, got {:?}", tok.kind)),
        }
    }

    fn parse_value(&mut self) -> Result<PrimeValue, String> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::StringLit => Ok(PrimeValue::String(tok.value)),
            TokenKind::NumberLit => {
                if tok.value.contains('.') {
                    Ok(PrimeValue::Float(
                        tok.value.parse().map_err(|e| format!("{}", e))?,
                    ))
                } else {
                    Ok(PrimeValue::Int(
                        tok.value.parse().map_err(|e| format!("{}", e))?,
                    ))
                }
            }
            TokenKind::BoolLit => Ok(PrimeValue::Bool(tok.value == "true")),
            TokenKind::Ident => Ok(PrimeValue::Variable(tok.value)),
            _ => Err(format!("expected value, got {:?}", tok.kind)),
        }
    }

    fn parse_data_map(&mut self) -> Result<HashMap<String, PrimeValue>, String> {
        let mut map = HashMap::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let key = self.expect(TokenKind::Ident)?.value;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_newlines();
            if self.check(TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }
        Ok(map)
    }

    fn parse_conditional(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::If)?;
        let left = self.parse_value()?;
        let op = self.parse_filter_op()?;
        let right = self.parse_value()?;
        let condition = Condition { left, op, right };
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let then = self.parse_statement()?;
        self.skip_newlines();
        self.expect(TokenKind::RBrace)?;
        let else_branch = if self.check(TokenKind::Else) {
            self.advance();
            self.expect(TokenKind::LBrace)?;
            self.skip_newlines();
            let e = self.parse_statement()?;
            self.skip_newlines();
            self.expect(TokenKind::RBrace)?;
            Some(Box::new(e))
        } else {
            None
        };
        Ok(PrimeNode::Conditional {
            condition,
            then: Box::new(then),
            else_branch,
        })
    }

    fn parse_loop(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::For)?;
        let variable = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::In)?;
        let collection = self.parse_value()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let body = self.parse_statement()?;
        self.skip_newlines();
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Loop {
            variable,
            collection,
            body: Box::new(body),
        })
    }

    fn parse_return(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Return)?;
        let value = self.parse_value()?;
        Ok(PrimeNode::Return { value })
    }

    fn parse_assign(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Let)?;
        let name = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_value()?;
        Ok(PrimeNode::Assign { name, value })
    }

    fn parse_api(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Api)?;
        let name = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut endpoints = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            endpoints.push(self.parse_endpoint()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Api { name, endpoints })
    }

    fn parse_endpoint(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Endpoint)?;
        let method_str = self.expect(TokenKind::Ident)?.value;
        let method = match method_str.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            _ => return Err(format!("unknown HTTP method: {}", method_str)),
        };
        let path = self.expect(TokenKind::StringLit)?.value;
        self.expect(TokenKind::Arrow)?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let handler = self.parse_statement()?;
        self.skip_newlines();
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Endpoint {
            method,
            path,
            params: vec![],
            handler: Box::new(handler),
        })
    }

    fn parse_call(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Call)?;
        let target = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::Dot)?;
        let method = self.expect(TokenKind::Ident)?.value;
        let mut args = Vec::new();
        if self.check(TokenKind::LParen) {
            self.advance();
            while !self.check(TokenKind::RParen) && !self.at_end() {
                args.push(self.parse_value()?);
                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen)?;
        }
        Ok(PrimeNode::Call {
            target,
            method,
            args,
        })
    }

    // --- Helper methods ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        if self.peek().kind == kind {
            Ok(self.advance())
        } else {
            Err(format!(
                "expected {:?}, got {:?} at {}:{}",
                kind,
                self.peek().kind,
                self.peek().line,
                self.peek().col
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while !self.at_end() && self.peek().kind == TokenKind::Newline {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::lexer::Lexer;

    #[test]
    fn test_parse_entity() {
        let tokens = Lexer::new("entity User { name: string, age: int }").tokenize();
        let mut parser = ScriptParser::new(tokens);
        let node = parser.parse().unwrap();
        assert_eq!(node.type_name(), "entity");
    }

    #[test]
    fn test_parse_create() {
        let tokens = Lexer::new("create User { name: \"Alice\", age: 30 }").tokenize();
        let mut parser = ScriptParser::new(tokens);
        let node = parser.parse().unwrap();
        assert_eq!(node.type_name(), "create");
    }

    #[test]
    fn test_parse_return() {
        let tokens = Lexer::new("return \"hello\"").tokenize();
        let mut parser = ScriptParser::new(tokens);
        let node = parser.parse().unwrap();
        assert_eq!(node.type_name(), "return");
    }
}
