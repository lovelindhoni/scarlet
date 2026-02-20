use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::scanner::{Scanner, Token, TokenType};

type ParseFn = fn(&mut Parser);

struct ParseRule {
    pub prefix: Option<ParseFn>,
    pub infix: Option<ParseFn>,
    pub precedence: Precedence,
}

impl ParseRule {
    pub fn new(prefix: Option<ParseFn>, infix: Option<ParseFn>, precedence: Precedence) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }
}

pub fn compile(source: Vec<u8>) -> Result<Chunk, String> {
    let mut parser = Parser::new(source);
    parser.advance();
    parser.expression();
    parser.consume(TokenType::Eof, "Expect end of expression");
    parser.end_compiler();
    if parser.had_error {
        return Err(format!("Compiliation Failed!"));
    } else {
        Ok(parser.get_chunk())
    }
}

struct Parser {
    previous: Option<Token>,
    current: Option<Token>,
    had_error: bool,
    panic_mode: bool,
    scanner: Scanner,
    chunk: Chunk,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Precedence {
    None = 0,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    pub fn next(self) -> Self {
        use Precedence::*;
        match self {
            None => Assignment,
            Assignment => Or,
            Or => And,
            And => Equality,
            Equality => Comparison,
            Comparison => Term,
            Term => Factor,
            Factor => Unary,
            Unary => Call,
            Call => Primary,
            Primary => Primary, // highest stays highest
        }
    }
}

impl Parser {
    fn get_rule(&self, token_variant: TokenType) -> ParseRule {
        match token_variant {
            TokenType::LeftParen => ParseRule::new(Some(Parser::grouping), None, Precedence::None),
            TokenType::Minus => {
                ParseRule::new(Some(Parser::unary), Some(Parser::binary), Precedence::Term)
            }
            TokenType::Plus => ParseRule::new(None, Some(Parser::binary), Precedence::Term),
            TokenType::Slash => ParseRule::new(None, Some(Parser::binary), Precedence::Factor),
            TokenType::Star => ParseRule::new(None, Some(Parser::binary), Precedence::Factor),
            TokenType::Modulo => ParseRule::new(None, Some(Parser::binary), Precedence::Factor),
            TokenType::Number => ParseRule::new(Some(Parser::number), None, Precedence::None),

            _ => ParseRule::new(None, None, Precedence::None),
        }
    }
    fn binary(&mut self) {
        let (variant, line) = {
            let prev = self
                .previous
                .as_ref()
                .expect("Binary called without previous token");
            (prev.variant, prev.line)
        };
        let rule = self.get_rule(variant);
        self.parse_precedence(rule.precedence.next());
        match variant {
            TokenType::Plus => self.chunk.write_instruction(Instruction::Add, line),
            TokenType::Minus => self.chunk.write_instruction(Instruction::Subtract, line),
            TokenType::Star => self.chunk.write_instruction(Instruction::Multiply, line),
            TokenType::Slash => self.chunk.write_instruction(Instruction::Divide, line),
            TokenType::Modulo => self.chunk.write_instruction(Instruction::Modulo, line),
            _ => {
                // not reachable yet
            }
        }
    }
    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        let prev_variant = self.previous.as_ref().expect("No previous token").variant;
        let rule = self.get_rule(prev_variant);
        if let Some(prefix_rule) = rule.prefix {
            prefix_rule(self);
        } else {
            self.error_at_previous("Expect expression.");
            return;
        }

        while {
            let curr_variant = match self.current.as_ref() {
                Some(token) => token.variant,
                None => return,
            };
            precedence <= self.get_rule(curr_variant).precedence
        } {
            self.advance();

            let prev_variant = self.previous.as_ref().unwrap().variant;
            let rule = self.get_rule(prev_variant);

            if let Some(infix_rule) = rule.infix {
                infix_rule(self);
            } else {
                self.error_at_previous("Expect infix operator.");
                return;
            }
        }
    }
    fn number(&mut self) {
        if let Some(previous_token) = self.previous.as_ref() {
            let value = str::from_utf8(&previous_token.lexeme)
                .expect("Previous token lexme is not valid utf8")
                .parse::<f64>()
                .expect("This is not a slice that can be parsed to f64");
            self.chunk
                .write_constant(Value::Number(value), previous_token.line);
        } else {
            eprintln!("Previous token not present here")
        }
    }
    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ) after expression");
    }
    fn unary(&mut self) {
        let variant = self
            .previous
            .as_ref()
            .expect("Binary called without previous token")
            .variant;
        let line = self
            .previous
            .as_ref()
            .expect("Binary called without previous token")
            .line;

        self.parse_precedence(Precedence::Unary);
        match variant {
            TokenType::Minus => {
                self.chunk.write_instruction(Instruction::Negate, line);
            }
            _ => {
                // not reachable yet
            }
        }
    }
    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }
    fn end_compiler(&mut self) {
        self.emit_return();
    }
    fn emit_return(&mut self) {
        if let Some(previous_token) = self.previous.as_ref() {
            self.chunk
                .write_instruction(Instruction::Return, previous_token.line);
        } else {
            eprintln!("Previous token not present here")
        }
    }
    pub fn new(source: Vec<u8>) -> Self {
        let scanner = Scanner::new(source);
        let chunk = Chunk::new("Master");
        Self {
            previous: None,
            current: None,
            had_error: false,
            panic_mode: false,
            scanner,
            chunk,
        }
    }
    fn consume(&mut self, token_variant: TokenType, message: &str) {
        if let Some(token) = self.current.as_ref() {
            if token_variant == token.variant {
                self.advance();
            } else {
                self.error_at_current(message);
            }
        }
    }
    fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.variant {
            TokenType::Eof => {
                eprint!(" at end");
            }
            _ => {
                eprint!(" at '{}'", str::from_utf8(&token.lexeme).unwrap());
            }
        }

        eprintln!(": {}", message);
        self.had_error = true;
    }
    fn error_at_previous(&mut self, message: &str) {
        if let Some(previous) = self.previous.clone() {
            self.error_at(previous, message);
        } else {
            eprintln!("Parser doesn't have previous token stored!");
        }
    }
    fn error_at_current(&mut self, message: &str) {
        if let Some(current) = self.previous.clone() {
            self.error_at(current, message);
        } else {
            eprintln!("Parser doesn't have current token stored!");
        }
    }
    fn advance(&mut self) {
        self.previous = self.current.clone();
        loop {
            self.current = Some(self.scanner.scan_token());
            let current_token = self.current.as_ref().expect("Should be a token here!");
            match current_token.variant {
                TokenType::UnterminatedString => {
                    self.error_at_current("Unterminated String");
                }
                TokenType::UnexpectedCharacter => {
                    self.error_at_current("Unexpected Character");
                }
                _ => {
                    break;
                }
            }
        }
    }
    pub fn get_chunk(self) -> Chunk {
        self.chunk
    }
}
