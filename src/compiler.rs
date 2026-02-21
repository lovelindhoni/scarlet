use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::scanner::{Scanner, Token, TokenType};

use anyhow::{Context, Result as AnyhowResult, anyhow};

type ParseFn = fn(&mut Parser) -> AnyhowResult<()>;

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

pub fn compile(source: Vec<u8>) -> AnyhowResult<Chunk> {
    let mut parser = Parser::new(source);
    parser.advance()?;
    parser.expression()?;
    parser.consume(TokenType::Eof, "Expect end of expression")?;
    parser.end_compiler()?;
    if parser.had_error {
        Err(anyhow!("Compiliation Failed!"))
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
    fn binary(&mut self) -> AnyhowResult<()> {
        let (variant, line) = {
            let prev = self
                .previous
                .as_ref()
                .context("Previous token not present here")?;
            (prev.variant, prev.line)
        };
        let rule = self.get_rule(variant);
        self.parse_precedence(rule.precedence.next())?;
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
        Ok(())
    }
    fn parse_precedence(&mut self, precedence: Precedence) -> AnyhowResult<()> {
        self.advance()?;

        let prev_variant = self.previous.as_ref().expect("No previous token").variant;
        let rule = self.get_rule(prev_variant);
        if let Some(prefix_rule) = rule.prefix {
            prefix_rule(self)?;
        } else {
            self.error_at_previous("Expect expression.")?;
            return Ok(());
        }

        while {
            let curr_variant = match self.current.as_ref() {
                Some(token) => token.variant,
                None => return Ok(()),
            };
            precedence <= self.get_rule(curr_variant).precedence
        } {
            self.advance()?;

            let prev_variant = self
                .previous
                .as_ref()
                .context("Previous token not present here")?
                .variant;
            let rule = self.get_rule(prev_variant);

            let infix_rule = rule
                .infix
                .context("Infix rule not present for this token variant here")?;
            infix_rule(self)?;
        }
        Ok(())
    }
    fn number(&mut self) -> AnyhowResult<()> {
        let previous_token = self
            .previous
            .as_ref()
            .context("Previous token not present here")?;
        let value_str = str::from_utf8(&previous_token.lexeme).with_context(|| {
            format!(
                "Previous token lexme is not valid utf8: {:?}",
                &previous_token.lexeme
            )
        })?;
        let value: f64 = value_str.parse().with_context(|| {
            format!(
                "This is not a lexme that can be parsed to f64: {:?}",
                value_str
            )
        })?;
        self.chunk
            .write_constant(Value::Number(value), previous_token.line);
        Ok(())
    }
    fn grouping(&mut self) -> AnyhowResult<()> {
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ) after expression")?;
        Ok(())
    }
    fn unary(&mut self) -> AnyhowResult<()> {
        let variant = self
            .previous
            .as_ref()
            .context("Previous token not present here")?
            .variant;
        let line = self
            .previous
            .as_ref()
            .context("Previous token not present here")?
            .line;

        self.parse_precedence(Precedence::Unary)?;
        match variant {
            TokenType::Minus => {
                self.chunk.write_instruction(Instruction::Negate, line);
            }
            _ => {
                // not reachable yet
            }
        }
        Ok(())
    }
    fn expression(&mut self) -> AnyhowResult<()> {
        self.parse_precedence(Precedence::Assignment)?;
        Ok(())
    }
    fn end_compiler(&mut self) -> AnyhowResult<()> {
        self.emit_return()?;
        Ok(())
    }
    fn emit_return(&mut self) -> AnyhowResult<()> {
        let previous_token = self
            .previous
            .as_ref()
            .context("Previous token not present when emitting return")?;
        self.chunk
            .write_instruction(Instruction::Return, previous_token.line);
        Ok(())
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
    fn consume(&mut self, token_variant: TokenType, message: &str) -> AnyhowResult<()> {
        if let Some(token) = self.current.as_ref() {
            if token_variant == token.variant {
                self.advance()?;
            } else {
                self.error_at_current(message)?;
            }
        }
        Ok(())
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
    fn error_at_previous(&mut self, message: &str) -> AnyhowResult<()> {
        let previous_token = self
            .previous
            .clone()
            .context("Previous token not present when error logging")?;
        self.error_at(previous_token, message);
        Ok(())
    }
    fn error_at_current(&mut self, message: &str) -> AnyhowResult<()> {
        let current_token = self
            .current
            .clone()
            .context("Current token not present when error logging")?;
        self.error_at(current_token, message);
        Ok(())
    }
    fn advance(&mut self) -> AnyhowResult<()> {
        self.previous = self.current.clone();
        loop {
            self.current = Some(self.scanner.scan_token());
            let current_token = self
                .current
                .as_ref()
                .context("Current token not present when advancing")?;
            match current_token.variant {
                TokenType::UnterminatedString => {
                    self.error_at_current("Unterminated String")?;
                }
                TokenType::UnexpectedCharacter => {
                    self.error_at_current("Unexpected Character")?;
                }
                _ => return Ok(()),
            }
        }
    }
    pub fn get_chunk(self) -> Chunk {
        self.chunk
    }
}
