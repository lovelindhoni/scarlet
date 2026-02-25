use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::{CompileError, HeapError};
use crate::heap::{Heap, Object};
use crate::scanner::{Scanner, Token, TokenType};

type Result<T> = std::result::Result<T, CompileError>;

pub fn compile(source: Vec<u8>, chunk: &mut Chunk, heap: &mut Heap) -> Result<()> {
    let mut parser = Parser::new(source, chunk, heap);
    parser.advance()?;
    parser.expression()?;
    parser.consume(TokenType::Eof, "Expect end of expression")?;
    parser.end_compiler()?;
    Ok(())
}

struct Parser<'a> {
    previous: Option<Token>,
    current: Option<Token>,
    scanner: Scanner,
    chunk: &'a mut Chunk,
    heap: &'a mut Heap,
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

impl<'a> Parser<'a> {
    fn get_rule_precedence(&self, token_variant: TokenType) -> Precedence {
        match token_variant {
            TokenType::Minus | TokenType::Plus => Precedence::Term,
            TokenType::Slash | TokenType::Star | TokenType::Modulo => Precedence::Factor,

            TokenType::BangEqual | TokenType::EqualEqual => Precedence::Equality,

            TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => Precedence::Comparison,

            _ => Precedence::None,
        }
    }
    fn execute_prefix_parser(&mut self, token_variant: TokenType) -> Result<()> {
        match token_variant {
            TokenType::LeftParen => self.grouping(),
            TokenType::Number => self.number(),
            TokenType::String => self.string(),
            TokenType::Minus | TokenType::Bang => self.unary(),
            TokenType::True | TokenType::False | TokenType::Nil => self.literal(),

            _ => Err(CompileError::MissingPrefixParser {
                message: "Expect expression".to_owned(),
                token: self
                    .current
                    .as_ref()
                    .ok_or(CompileError::MissingCurrentToken)?
                    .clone(),
            }),
        }
    }

    fn execute_infix_parser(&mut self, token_variant: TokenType) -> Result<()> {
        match token_variant {
            TokenType::Minus
            | TokenType::Plus
            | TokenType::Slash
            | TokenType::Star
            | TokenType::Modulo
            | TokenType::BangEqual
            | TokenType::EqualEqual
            | TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => self.binary(),

            _ => {
                let prev_variant = self
                    .previous
                    .as_ref()
                    .ok_or(CompileError::MissingPreviousToken)?
                    .variant;

                Err(CompileError::MissingInfixParser(prev_variant))
            }
        }
    }
    fn string(&mut self) -> Result<()> {
        let previous_token = self
            .previous
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?;

        let lexeme = &previous_token.lexeme;
        let trimmed_lexeme = &lexeme[1..lexeme.len() - 1];
        let string_value = String::from_utf8_lossy(trimmed_lexeme).to_string();
        let key = if self.heap.intern_table.contains_key(&string_value) {
            println!("hi");
            let interned_key = self.heap.intern_table[&string_value];
            let object = self
                .heap
                .arena
                .get(interned_key)
                .ok_or(HeapError::ExpiredArenaKey)?;
            match object {
                Object::String { value } => {
                    if value != &string_value {
                        return Err(HeapError::InvalidInternedKey {
                            expected: string_value,
                            found: value.to_owned(),
                        }
                        .into());
                    }
                }
            }
            interned_key
        } else {
            let interned_key = self.heap.arena.insert(Object::String {
                value: string_value.clone(),
            });
            self.heap.intern_table.insert(string_value, interned_key);
            interned_key
        };

        self.chunk
            .write_constant(Value::Object(key), previous_token.line);

        Ok(())
    }
    fn literal(&mut self) -> Result<()> {
        let previous_token = self
            .previous
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?;
        match previous_token.variant {
            TokenType::True => self
                .chunk
                .write_instruction(Instruction::True, previous_token.line),
            TokenType::False => self
                .chunk
                .write_instruction(Instruction::False, previous_token.line),
            TokenType::Nil => self
                .chunk
                .write_instruction(Instruction::Nil, previous_token.line),
            _ => {
                // unreachable
            }
        }
        Ok(())
    }
    fn binary(&mut self) -> Result<()> {
        let (variant, line) = {
            let prev = self
                .previous
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?;
            (prev.variant, prev.line)
        };
        let rule = self.get_rule_precedence(variant);
        self.parse_precedence(rule.next())?;
        match variant {
            TokenType::Plus => self.chunk.write_instruction(Instruction::Add, line),
            TokenType::Minus => self.chunk.write_instruction(Instruction::Subtract, line),
            TokenType::Star => self.chunk.write_instruction(Instruction::Multiply, line),
            TokenType::Slash => self.chunk.write_instruction(Instruction::Divide, line),
            TokenType::Modulo => self.chunk.write_instruction(Instruction::Modulo, line),
            TokenType::BangEqual => {
                self.chunk.write_instruction(Instruction::Equal, line);
                self.chunk.write_instruction(Instruction::Not, line);
            }
            TokenType::EqualEqual => self.chunk.write_instruction(Instruction::Equal, line),
            TokenType::Greater => self.chunk.write_instruction(Instruction::Greater, line),
            TokenType::GreaterEqual => {
                self.chunk.write_instruction(Instruction::Less, line);
                self.chunk.write_instruction(Instruction::Not, line);
            }
            TokenType::Less => self.chunk.write_instruction(Instruction::Less, line),
            TokenType::LessEqual => {
                self.chunk.write_instruction(Instruction::Greater, line);
                self.chunk.write_instruction(Instruction::Not, line);
            }

            _ => {
                // not reachable yet
            }
        }
        Ok(())
    }
    fn parse_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.advance()?;

        let prev_variant = self.previous.as_ref().expect("No previous token").variant;
        self.execute_prefix_parser(prev_variant)?;

        while {
            let curr_variant = self
                .current
                .as_ref()
                .ok_or(CompileError::MissingCurrentToken)?
                .variant;
            precedence <= self.get_rule_precedence(curr_variant)
        } {
            self.advance()?;

            let prev_variant = self
                .previous
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?
                .variant;
            self.execute_infix_parser(prev_variant)?;
        }
        Ok(())
    }
    fn number(&mut self) -> Result<()> {
        let previous_token = self
            .previous
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?;
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
                .ok_or(CompileError::MissingPreviousToken)?;
            (prev.variant, prev.line)
        };

        self.parse_precedence(Precedence::Unary)?;
        match variant {
            TokenType::Minus => {
                self.chunk.write_instruction(Instruction::Negate, line);
            }
            TokenType::Bang => {
                self.chunk.write_instruction(Instruction::Not, line);
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
            .ok_or(CompileError::MissingPreviousToken)?;
        self.chunk
            .write_instruction(Instruction::Return, previous_token.line);
        Ok(())
    }
    pub fn new(source: Vec<u8>, chunk: &'a mut Chunk, heap: &'a mut Heap) -> Self {
        let scanner = Scanner::new(source);
        Self {
            previous: None,
            current: None,
            scanner,
            chunk,
            heap,
        }
    }
    fn consume(&mut self, token_variant: TokenType, message: &str) -> Result<()> {
        let token = self
            .current
            .as_ref()
            .ok_or(CompileError::MissingCurrentToken)?;
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
}
