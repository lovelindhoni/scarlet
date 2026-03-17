use crate::error::ScanError;
use std::fmt;

pub struct Scanner {
    start: usize,   // start of current lexeme
    current: usize, // current character
    line: u64,      // what line current lexeme is on?
    source: Vec<u8>,
}

fn is_digit(byte: &u8) -> bool {
    byte.is_ascii_digit()
}

fn is_alpha(byte: &u8) -> bool {
    byte.is_ascii_alphabetic() || byte == &b'_'
}

impl Scanner {
    pub fn new(source: Vec<u8>) -> Self {
        Self {
            start: 0,
            current: 0,
            line: 1,
            source,
        }
    }
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn make_token(&self, token: TokenType) -> Token {
        Token {
            variant: token,
            lexeme: self.source[self.start..self.current].to_vec(),
            line: self.line,
        }
    }

    fn skip_ignorables(&mut self) {
        // skips whitespace, comments
        loop {
            if self.is_at_end() {
                return;
            }
            match self.peek() {
                b' ' | b'\r' | b'\t' => {
                    self.advance();
                }
                b'\n' => {
                    self.line += 1;
                    self.advance();
                }
                b'/' => {
                    if self.current + 1 < self.source.len() && self.source[self.current + 1] == b'/'
                    {
                        while !self.is_at_end() && self.peek() != b'\n' {
                            self.advance();
                        }
                    } else {
                        break;
                    }
                }
                _ => return,
            }
        }
    }

    fn peek(&self) -> u8 {
        self.source[self.current]
    }

    fn advance(&mut self) -> usize {
        let current = self.current;
        self.current += 1;
        current
    }

    fn match_next(&mut self, expected: u8) -> bool {
        if self.is_at_end() || self.peek() != expected {
            return false;
        };
        self.current += 1;
        true
    }

    fn check_keyword(
        &self,
        start: usize,
        length: usize,
        rest: &str,
        token_type: TokenType,
    ) -> TokenType {
        if self.current - self.start == start + length
            && &self.source[self.start + start..self.start + start + length] == rest.as_bytes()
        {
            token_type
        } else {
            TokenType::Identifier
        }
    }

    pub fn scan_token(&mut self) -> Result<Token, ScanError> {
        self.skip_ignorables();
        self.start = self.current;
        if self.is_at_end() {
            return Ok(self.make_token(TokenType::Eof));
        }

        let current_i = self.advance();
        let c = self.source[current_i];

        match c {
            c if is_alpha(&c) => {
                while !self.is_at_end() && (is_alpha(&self.peek()) || is_digit(&self.peek())) {
                    self.advance();
                }
                let identifier = match self.source[self.start] {
                    b'a' => self.check_keyword(1, 2, "nd", TokenType::And),
                    b'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
                    b'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
                    b'o' => self.check_keyword(1, 1, "r", TokenType::Or),
                    b'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
                    b'w' => self.check_keyword(1, 4, "hile", TokenType::While),
                    b'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
                    b's' => self.check_keyword(1, 4, "uper", TokenType::Super),
                    b'l' => self.check_keyword(1, 2, "et", TokenType::Let),
                    b'i' => {
                        if self.current - self.start > 1 {
                            match self.source[self.start + 1] {
                                b'f' => self.check_keyword(2, 0, "", TokenType::If),
                                b'n' => self.check_keyword(2, 6, "herits", TokenType::Inherits),
                                _ => TokenType::Identifier,
                            }
                        } else {
                            TokenType::Identifier
                        }
                    }
                    b'f' => {
                        if self.current - self.start > 1 {
                            match self.source[self.start + 1] {
                                b'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                                b'o' => self.check_keyword(2, 1, "r", TokenType::For),
                                b'u' => self.check_keyword(2, 1, "n", TokenType::Fun),
                                _ => TokenType::Identifier,
                            }
                        } else {
                            TokenType::Identifier
                        }
                    }
                    b't' => {
                        if self.current - self.start > 1 {
                            match self.source[self.start + 1] {
                                b'h' => self.check_keyword(2, 2, "is", TokenType::This),
                                b'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                                _ => TokenType::Identifier,
                            }
                        } else {
                            TokenType::Identifier
                        }
                    }

                    _ => TokenType::Identifier,
                };
                Ok(self.make_token(identifier))
            }
            c if is_digit(&c) => {
                while !self.is_at_end() && is_digit(&self.peek()) {
                    self.advance();
                }
                if !self.is_at_end()
                    && self.peek() == b'.'
                    && self.current + 1 < self.source.len()
                    && is_digit(&self.source[self.current + 1])
                {
                    self.advance();
                    while !self.is_at_end() && is_digit(&self.peek()) {
                        self.advance();
                    }
                }
                Ok(self.make_token(TokenType::Number))
            }
            b'(' => Ok(self.make_token(TokenType::LeftParen)),
            b')' => Ok(self.make_token(TokenType::RightParen)),
            b'{' => Ok(self.make_token(TokenType::LeftBrace)),
            b'}' => Ok(self.make_token(TokenType::RightBrace)),
            b';' => Ok(self.make_token(TokenType::Semicolon)),
            b',' => Ok(self.make_token(TokenType::Comma)),
            b'.' => Ok(self.make_token(TokenType::Dot)),
            b'-' => Ok(self.make_token(TokenType::Minus)),
            b'+' => Ok(self.make_token(TokenType::Plus)),
            b'/' => Ok(self.make_token(TokenType::Slash)),
            b'*' => Ok(self.make_token(TokenType::Star)),
            b'%' => Ok(self.make_token(TokenType::Modulo)),

            b'!' => {
                let variant = if self.match_next(b'=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                Ok(self.make_token(variant))
            }
            b'=' => {
                let variant = if self.match_next(b'=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                Ok(self.make_token(variant))
            }
            b'<' => {
                let variant = if self.match_next(b'=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                Ok(self.make_token(variant))
            }
            b'>' => {
                let variant = if self.match_next(b'=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                Ok(self.make_token(variant))
            }

            b'"' => {
                while !self.is_at_end() && self.peek() != b'"' {
                    if self.peek() == b'\n' {
                        self.line += 1;
                    }
                    self.advance();
                }
                if self.is_at_end() {
                    return Err(ScanError::UnterminatedString {
                        message: String::from("Unterminated String"),
                        line: self.line,
                        lexeme: self.source[self.start..self.current].to_owned(),
                    });
                }
                self.advance(); // closing double quote
                Ok(self.make_token(TokenType::String))
            }

            _ => Err(ScanError::UnexpectedCharacter {
                message: String::from("Unexpected Character"),
                line: self.line,
                lexeme: self.source[self.start..self.current].to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Modulo,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Return,
    Super,
    Class,
    This,
    True,
    Let,
    While,
    Inherits,

    Eof,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            // Single-character tokens
            TokenType::LeftParen => "(",
            TokenType::RightParen => ")",
            TokenType::LeftBrace => "{",
            TokenType::RightBrace => "}",
            TokenType::Comma => ",",
            TokenType::Dot => ".",
            TokenType::Minus => "-",
            TokenType::Plus => "+",
            TokenType::Semicolon => ";",
            TokenType::Slash => "/",
            TokenType::Star => "*",
            TokenType::Modulo => "%",

            // One or two character tokens
            TokenType::Bang => "!",
            TokenType::BangEqual => "!=",
            TokenType::Equal => "=",
            TokenType::EqualEqual => "==",
            TokenType::Greater => ">",
            TokenType::GreaterEqual => ">=",
            TokenType::Less => "<",
            TokenType::LessEqual => "<=",

            // Literals
            TokenType::Identifier => "identifier",
            TokenType::String => "string",
            TokenType::Number => "number",

            // Keywords
            TokenType::And => "and",
            TokenType::Else => "else",
            TokenType::False => "false",
            TokenType::For => "for",
            TokenType::Fun => "fun",
            TokenType::Class => "class",
            TokenType::If => "if",
            TokenType::Nil => "nil",
            TokenType::Or => "or",
            TokenType::Return => "return",
            TokenType::Super => "super",
            TokenType::This => "this",
            TokenType::True => "true",
            TokenType::Let => "let",
            TokenType::While => "while",
            TokenType::Inherits => "inherits",

            TokenType::Eof => "end of file",
        };

        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub variant: TokenType,
    pub lexeme: Vec<u8>,
    pub line: u64,
}
