use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::CompileError;
use crate::scanner::{Scanner, Token, TokenType};

type Result<T> = std::result::Result<T, CompileError>;
type ParseFn = fn(&mut Parser) -> Result<()>;

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

pub fn compile(source: Vec<u8>) -> Result<Chunk> {
    let mut parser = Parser::new(source);
    parser.advance()?;
    parser.expression()?;
    parser.consume(TokenType::Eof, "Expect end of expression")?;
    parser.end_compiler()?;
    Ok(parser.get_chunk())
}

struct Parser {
    previous: Option<Token>,
    current: Option<Token>,
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
    fn binary(&mut self) -> Result<()> {
        let (variant, line) = {
            let prev = self
                .previous
                .as_ref()
                .ok_or(CompileError::PreviousTokenAbsence)?;
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
    fn parse_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.advance()?;

        let prev_variant = self.previous.as_ref().expect("No previous token").variant;
        let rule = self.get_rule(prev_variant);
        if let Some(prefix_rule) = rule.prefix {
            prefix_rule(self)?;
        } else {
            return Err(CompileError::PrefixParserAbsence {
                message: "Expect expression".to_owned(),
                token: self
                    .current
                    .as_ref()
                    .ok_or(CompileError::CurrentTokenAbsence)?
                    .clone(),
            });
        }

        while {
            let curr_variant = self
                .current
                .as_ref()
                .ok_or(CompileError::CurrentTokenAbsence)?
                .variant;
            precedence <= self.get_rule(curr_variant).precedence
        } {
            self.advance()?;

            let prev_variant = self
                .previous
                .as_ref()
                .ok_or(CompileError::PreviousTokenAbsence)?
                .variant;
            let rule = self.get_rule(prev_variant);

            let infix_rule = rule
                .infix
                .ok_or(CompileError::InfixParserAbsence(prev_variant))?;
            infix_rule(self)?;
        }
        Ok(())
    }
    fn number(&mut self) -> Result<()> {
        let previous_token = self
            .previous
            .as_ref()
            .ok_or(CompileError::PreviousTokenAbsence)?;
        let value_str = str::from_utf8(&previous_token.lexeme)
            .map_err(|e| CompileError::InvalidUtf8 { source: e })?;
        let value: f64 = value_str.parse().map_err(|e| CompileError::LiteralParse {
            literal: value_str.to_owned(),
            to: "Double".to_owned(),
            source: e,
        })?;

        self.chunk
            .write_constant(Value::Number(value), previous_token.line);
        Ok(())
    }
    fn grouping(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ) after expression")?;
        Ok(())
    }
    fn unary(&mut self) -> Result<()> {
        let (variant, line) = {
            let prev = self
                .previous
                .as_ref()
                .ok_or(CompileError::PreviousTokenAbsence)?;
            (prev.variant, prev.line)
        };

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
    fn expression(&mut self) -> Result<()> {
        self.parse_precedence(Precedence::Assignment)?;
        Ok(())
    }
    fn end_compiler(&mut self) -> Result<()> {
        self.emit_return()?;
        Ok(())
    }
    fn emit_return(&mut self) -> Result<()> {
        let previous_token = self
            .previous
            .as_ref()
            .ok_or(CompileError::PreviousTokenAbsence)?;
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
            scanner,
            chunk,
        }
    }
    fn consume(&mut self, token_variant: TokenType, message: &str) -> Result<()> {
        let token = self
            .current
            .as_ref()
            .ok_or(CompileError::CurrentTokenAbsence)?;
        if token_variant == token.variant {
            self.advance()?;
        } else {
            return Err(CompileError::UnexpectedToken {
                message: message.to_owned(),
                token: token.clone(),
            });
        }
        Ok(())
    }
    fn advance(&mut self) -> Result<()> {
        self.previous = self.current.clone();
        self.current = Some(self.scanner.scan_token()?);
        Ok(())
    }
    pub fn get_chunk(self) -> Chunk {
        self.chunk
    }
}
