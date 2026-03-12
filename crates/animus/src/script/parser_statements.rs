//! ScriptParser — statement-level parse methods.

use super::lexer::TokenKind;
use crate::prime::ast::*;

impl super::parser::ScriptParser {
    pub(crate) fn parse_entity(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_create(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Create)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let data = self.parse_data_map()?;
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Create { entity, data })
    }

    pub(crate) fn parse_read(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_update(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_delete(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Delete)?;
        let entity = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let filter = self.parse_filter()?;
        self.expect(TokenKind::RBrace)?;
        Ok(PrimeNode::Delete { entity, filter })
    }

    pub(crate) fn parse_conditional(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_loop(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_return(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Return)?;
        let value = self.parse_value()?;
        Ok(PrimeNode::Return { value })
    }

    pub(crate) fn parse_assign(&mut self) -> Result<PrimeNode, String> {
        self.expect(TokenKind::Let)?;
        let name = self.expect(TokenKind::Ident)?.value;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_value()?;
        Ok(PrimeNode::Assign { name, value })
    }

    pub(crate) fn parse_api(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_endpoint(&mut self) -> Result<PrimeNode, String> {
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

    pub(crate) fn parse_call(&mut self) -> Result<PrimeNode, String> {
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
}

#[cfg(test)]
mod tests {
    use crate::script::lexer::Lexer;
    use crate::script::parser::ScriptParser;

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
