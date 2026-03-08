//! Lexer — tokenize Animus Script source text.

/// Token produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub line: usize,
    pub col: usize,
}

/// Token kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Entity,
    Create,
    Read,
    Update,
    Delete,
    If,
    Else,
    For,
    In,
    Return,
    Endpoint,
    Api,
    Call,
    Let,
    Store,

    // Literals
    Ident,
    StringLit,
    NumberLit,
    BoolLit,

    // Symbols
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Arrow, // ->
    Dot,
    Eq,    // =
    EqEq,  // ==
    NotEq, // !=
    Lt,
    Gt,
    LtEq,
    GtEq,
    Semicolon,
    At, // @ for decorators

    // Special
    Newline,
    Eof,
}

/// Lexer for Animus Script
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Tokenize the entire source
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        self.skip_comments();
        self.skip_whitespace();

        if self.pos >= self.source.len() {
            return self.make_token(TokenKind::Eof, "");
        }

        let ch = self.source[self.pos];
        let line = self.line;
        let col = self.col;

        // Newlines
        if ch == '\n' {
            self.advance();
            return Token {
                kind: TokenKind::Newline,
                value: "\n".into(),
                line,
                col,
            };
        }

        // String literals
        if ch == '"' {
            return self.read_string();
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.read_number();
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.read_ident();
        }

        // Two-char symbols
        if self.pos + 1 < self.source.len() {
            let next = self.source[self.pos + 1];
            let two = format!("{}{}", ch, next);
            let kind = match two.as_str() {
                "->" => Some(TokenKind::Arrow),
                "==" => Some(TokenKind::EqEq),
                "!=" => Some(TokenKind::NotEq),
                "<=" => Some(TokenKind::LtEq),
                ">=" => Some(TokenKind::GtEq),
                _ => None,
            };
            if let Some(k) = kind {
                self.advance();
                self.advance();
                return Token {
                    kind: k,
                    value: two,
                    line,
                    col,
                };
            }
        }

        // Single-char symbols
        let kind = match ch {
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '=' => TokenKind::Eq,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            ';' => TokenKind::Semicolon,
            '@' => TokenKind::At,
            _ => {
                self.advance();
                return self.next_token(); // skip unknown chars
            }
        };
        self.advance();
        Token {
            kind,
            value: ch.to_string(),
            line,
            col,
        }
    }

    fn read_string(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        self.advance(); // skip opening "
        let mut s = String::new();
        while self.pos < self.source.len() && self.source[self.pos] != '"' {
            if self.source[self.pos] == '\\' && self.pos + 1 < self.source.len() {
                self.advance();
                match self.source[self.pos] {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    c => {
                        s.push('\\');
                        s.push(c);
                    }
                }
            } else {
                s.push(self.source[self.pos]);
            }
            self.advance();
        }
        if self.pos < self.source.len() {
            self.advance();
        } // skip closing "
        Token {
            kind: TokenKind::StringLit,
            value: s,
            line,
            col,
        }
    }

    fn read_number(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        while self.pos < self.source.len()
            && (self.source[self.pos].is_ascii_digit() || self.source[self.pos] == '.')
        {
            s.push(self.source[self.pos]);
            self.advance();
        }
        Token {
            kind: TokenKind::NumberLit,
            value: s,
            line,
            col,
        }
    }

    fn read_ident(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        while self.pos < self.source.len()
            && (self.source[self.pos].is_alphanumeric() || self.source[self.pos] == '_')
        {
            s.push(self.source[self.pos]);
            self.advance();
        }
        let kind = match s.as_str() {
            "entity" => TokenKind::Entity,
            "create" => TokenKind::Create,
            "read" => TokenKind::Read,
            "update" => TokenKind::Update,
            "delete" => TokenKind::Delete,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "endpoint" => TokenKind::Endpoint,
            "api" => TokenKind::Api,
            "call" => TokenKind::Call,
            "let" => TokenKind::Let,
            "store" => TokenKind::Store,
            "true" | "false" => TokenKind::BoolLit,
            _ => TokenKind::Ident,
        };
        Token {
            kind,
            value: s,
            line,
            col,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comments(&mut self) {
        if self.pos + 1 < self.source.len()
            && self.source[self.pos] == '/'
            && self.source[self.pos + 1] == '/'
        {
            while self.pos < self.source.len() && self.source[self.pos] != '\n' {
                self.advance();
            }
        }
    }

    fn advance(&mut self) {
        if self.pos < self.source.len() {
            if self.source[self.pos] == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn make_token(&self, kind: TokenKind, value: &str) -> Token {
        Token {
            kind,
            value: value.into(),
            line: self.line,
            col: self.col,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_entity() {
        let mut lexer = Lexer::new("entity User { name: string }");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Entity);
        assert_eq!(tokens[1].kind, TokenKind::Ident);
        assert_eq!(tokens[1].value, "User");
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
    }

    #[test]
    fn test_tokenize_string() {
        let mut lexer = Lexer::new("\"hello world\"");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].kind, TokenKind::StringLit);
        assert_eq!(tokens[0].value, "hello world");
    }

    #[test]
    fn test_tokenize_keywords() {
        let mut lexer = Lexer::new("create read update delete if else for in return");
        let tokens = lexer.tokenize();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&TokenKind::Create));
        assert!(kinds.contains(&&TokenKind::Read));
        assert!(kinds.contains(&&TokenKind::If));
        assert!(kinds.contains(&&TokenKind::Return));
    }

    #[test]
    fn test_tokenize_symbols() {
        let mut lexer = Lexer::new("-> == != <= >=");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Arrow);
        assert_eq!(tokens[1].kind, TokenKind::EqEq);
        assert_eq!(tokens[2].kind, TokenKind::NotEq);
    }
}
