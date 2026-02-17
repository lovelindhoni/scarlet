pub struct Scanner<'a> {
    start: usize,   // start of current lexme
    current: usize, // current character
    line: usize,    // what line current lexme is on?
    source: &'a [u8],
}

fn is_digit(byte: &u8) -> bool {
    (b'0'..=b'9').contains(byte)
}

fn is_alpha(byte: &u8) -> bool {
    byte.is_ascii_alphabetic() || byte == &b'_'
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a [u8]) -> Self {
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
            length: self.current - self.start,
            line: self.line,
            start: self.start,
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

    pub fn scan_token(&mut self) -> Token {
        self.skip_ignorables();
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let c = self.source[self.advance()];

        match c {
            c if is_alpha(&c) => {
                while !self.is_at_end() && (is_alpha(&self.peek()) || is_digit(&self.peek())) {
                    self.advance();
                }
                let identifier = match self.source[self.start] {
                    b'a' => self.check_keyword(1, 2, "nd", TokenType::And),
                    b'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
                    b'o' => self.check_keyword(1, 1, "r", TokenType::Or),
                    b'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
                    b'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
                    b'w' => self.check_keyword(1, 4, "hile", TokenType::While),
                    b'i' => self.check_keyword(1, 1, "f", TokenType::If),
                    b'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
                    b's' => self.check_keyword(1, 4, "uper", TokenType::Super),
                    b'l' => self.check_keyword(1, 2, "et", TokenType::Let),
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
                self.make_token(identifier)
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
                self.make_token(TokenType::Number)
            }
            b'(' => return self.make_token(TokenType::LeftParen),
            b')' => return self.make_token(TokenType::RightParen),
            b'{' => return self.make_token(TokenType::LeftBrace),
            b'}' => return self.make_token(TokenType::RightBrace),
            b';' => return self.make_token(TokenType::Semicolon),
            b',' => return self.make_token(TokenType::Comma),
            b'.' => return self.make_token(TokenType::Dot),
            b'-' => return self.make_token(TokenType::Minus),
            b'+' => return self.make_token(TokenType::Plus),
            b'/' => return self.make_token(TokenType::Slash),
            b'*' => return self.make_token(TokenType::Star),
            b'%' => return self.make_token(TokenType::Modulo),

            b'!' => {
                let variant = if self.match_next(b'=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                return self.make_token(variant);
            }
            b'=' => {
                let variant = if self.match_next(b'=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                return self.make_token(variant);
            }
            b'<' => {
                let variant = if self.match_next(b'=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                return self.make_token(variant);
            }
            b'>' => {
                let variant = if self.match_next(b'=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                return self.make_token(variant);
            }

            b'"' => {
                while !self.is_at_end() && self.peek() != b'"' {
                    if self.peek() == b'\n' {
                        self.line += 1;
                    }
                    self.advance();
                }
                if self.is_at_end() {
                    return self.make_token(TokenType::UnterminatedString);
                }
                self.advance(); // closing double quote
                self.make_token(TokenType::String)
            }

            _ => return self.make_token(TokenType::UnexpectedCharacter),
        }
    }
}

#[derive(Debug)]
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
    Print,
    Return,
    Super,
    This,
    True,
    Let,
    While,

    UnexpectedCharacter,
    UnterminatedString,
    Eof,
}

#[derive(Debug)]
pub struct Token {
    pub variant: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}
