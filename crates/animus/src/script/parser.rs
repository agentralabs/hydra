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

    pub(crate) fn parse_statement(&mut self) -> Result<PrimeNode, String> {
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

    // --- Value and filter parsing ---

    pub(crate) fn parse_filter(&mut self) -> Result<Filter, String> {
        let field = self.expect(TokenKind::Ident)?.value;
        let op = self.parse_filter_op()?;
        let value = self.parse_value()?;
        Ok(Filter { field, op, value })
    }

    pub(crate) fn parse_filter_op(&mut self) -> Result<FilterOp, String> {
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

    pub(crate) fn parse_value(&mut self) -> Result<PrimeValue, String> {
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

    pub(crate) fn parse_data_map(&mut self) -> Result<HashMap<String, PrimeValue>, String> {
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

    // --- Helper methods ---

    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    pub(crate) fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }

    pub(crate) fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        self.pos += 1;
        tok
    }

    pub(crate) fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
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

    pub(crate) fn skip_newlines(&mut self) {
        while !self.at_end() && self.peek().kind == TokenKind::Newline {
            self.advance();
        }
    }
}
