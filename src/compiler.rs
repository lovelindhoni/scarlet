use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::{CompileError, HeapError};
use crate::heap::{Heap, Object};
use crate::scanner::{Scanner, Token, TokenType};

type Result<T> = std::result::Result<T, CompileError>;

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

pub fn compile(source: Vec<u8>, chunk: &mut Chunk, heap: &mut Heap) -> Result<()> {
    let mut parser = Parser::new(source, chunk, heap);
    parser.advance()?;
    while !parser.match_token(TokenType::Eof)? {
        parser.declaration()?;
    }
    parser.end_compiler()?;
    Ok(())
}

struct Local {
    token: Token,
    depth: i64,
}

impl Local {
    pub fn new(token: Token, depth: i64) -> Self {
        Self { token, depth }
    }
}

struct Compiler {
    pub locals: Vec<Local>,
    pub scope_depth: i64,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
}

struct Parser<'a> {
    previous_token: Option<Token>,
    current_token: Option<Token>,
    scanner: Scanner,
    current: Compiler,
    chunk: &'a mut Chunk,
    heap: &'a mut Heap,
}

impl<'a> Parser<'a> {
    fn identifier_idx(&mut self, lexeme: Vec<u8>) -> usize {
        let identifier = String::from_utf8_lossy(&lexeme).to_string();
        let key = if self.heap.intern_table.contains_key(&identifier) {
            self.heap.intern_table[&identifier]
        } else {
            let interned_key = self.heap.arena.insert(Object::String {
                value: identifier.clone(),
            });
            self.heap.intern_table.insert(identifier, interned_key);
            interned_key
        };
        self.chunk.add_constant(Value::Object(key))
    }

    pub fn match_token(&mut self, token_variant: TokenType) -> Result<bool> {
        Ok(if !self.check(token_variant)? {
            false
        } else {
            self.advance()?;
            true
        })
    }
    fn check(&self, token_variant: TokenType) -> Result<bool> {
        Ok(self
            .current_token
            .as_ref()
            .ok_or(CompileError::MissingCurrentToken)?
            .variant
            == token_variant)
    }
    pub fn declaration(&mut self) -> Result<()> {
        if self.match_token(TokenType::Let)? {
            self.let_declaration()?;
        } else {
            self.statement()?;
        }
        Ok(())
    }
    fn parse_variable(&mut self, message: &str) -> Result<usize> {
        self.consume(TokenType::Identifier, message)?;
        self.declare_variable()?;
        if self.current.scope_depth > 0 {
            Ok(0)
        } else {
            let previous = self
                .previous_token
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?;
            Ok(self.identifier_idx(previous.lexeme.to_owned()))
        }
    }
    fn declare_variable(&mut self) -> Result<()> {
        // only for local variables
        if self.current.scope_depth > 0 {
            let token = self
                .previous_token
                .to_owned()
                .ok_or(CompileError::MissingPreviousToken)?;
            for local in self.current.locals.iter().rev() {
                if local.depth != -1 && local.depth < self.current.scope_depth {
                    break;
                }
                if self.identifiers_equal(&token, &local.token) {
                    let current = self
                        .current_token
                        .to_owned()
                        .ok_or(CompileError::MissingPreviousToken)?;
                    return Err(CompileError::RedefinitionOfLocalVar { token: current });
                }
            }
            self.add_local(token);
        }
        Ok(())
    }
    fn identifiers_equal(&self, a: &Token, b: &Token) -> bool {
        a.lexeme == b.lexeme
    }
    fn add_local(&mut self, token: Token) {
        self.current.locals.push(Local::new(token, -1));
    }
    fn define_variable(&mut self, global: usize) -> Result<()> {
        if self.current.scope_depth == 0 {
            // only for global variables
            let line = self
                .previous_token
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?
                .line;
            self.chunk
                .write_instruction(Instruction::DefineGlobal(global), line);
        } else {
            self.mark_intialized()?;
        }
        Ok(())
    }
    fn mark_intialized(&mut self) -> Result<()> {
        self.current
            .locals
            .last_mut()
            .ok_or(CompileError::LocalsEmpty)?
            .depth = self.current.scope_depth;
        Ok(())
    }
    fn let_declaration(&mut self) -> Result<()> {
        let variable = self.parse_variable("Expect variable name")?;
        if self.match_token(TokenType::Equal)? {
            self.expression()?;
        } else {
            let line = self
                .previous_token
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?
                .line;
            self.chunk.write_instruction(Instruction::Nil, line);
        }
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration",
        )?;
        self.define_variable(variable)?;
        Ok(())
    }
    fn statement(&mut self) -> Result<()> {
        if self.match_token(TokenType::Print)? {
            self.print_statement()?;
        } else if self.match_token(TokenType::LeftBrace)? {
            self.begin_scope();
            self.block()?;
            self.end_scope()?;
        } else {
            self.expression_statement()?;
        }
        Ok(())
    }
    fn block(&mut self) -> Result<()> {
        while !self.check(TokenType::RightBrace)? && !self.check(TokenType::Eof)? {
            self.declaration()?;
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block")?;
        Ok(())
    }
    fn begin_scope(&mut self) {
        self.current.scope_depth += 1;
    }
    fn end_scope(&mut self) -> Result<()> {
        self.current.scope_depth -= 1;
        while let Some(local) = self.current.locals.last() {
            if local.depth <= self.current.scope_depth {
                break;
            }
            let line = self
                .previous_token
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?
                .line;

            self.chunk.write_instruction(Instruction::Pop, line);
            self.current.locals.pop();
        }

        Ok(())
    }
    fn expression_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression")?;
        let line = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?
            .line;
        self.chunk.write_instruction(Instruction::Pop, line);
        Ok(())
    }
    fn print_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value")?;
        let line = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?
            .line;
        self.chunk.write_instruction(Instruction::Print, line);
        Ok(())
    }
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
    fn execute_prefix_parser(&mut self, token_variant: TokenType, can_assign: bool) -> Result<()> {
        match token_variant {
            TokenType::LeftParen => self.grouping(),
            TokenType::Number => self.number(),
            TokenType::String => self.string(),
            TokenType::Identifier => self.variable(can_assign),
            TokenType::Minus | TokenType::Bang => self.unary(),
            TokenType::True | TokenType::False | TokenType::Nil => self.literal(),

            _ => Err(CompileError::MissingPrefixParser {
                message: "Expect expression".to_owned(),
                token: self
                    .current_token
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
                    .previous_token
                    .as_ref()
                    .ok_or(CompileError::MissingPreviousToken)?
                    .variant;

                Err(CompileError::MissingInfixParser(prev_variant))
            }
        }
    }

    fn variable(&mut self, can_assign: bool) -> Result<()> {
        let previous = self
            .previous_token
            .to_owned()
            .ok_or(CompileError::MissingPreviousToken)?;
        self.named_variable(previous, can_assign)?;
        Ok(())
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) -> Result<()> {
        let line = token.line;
        if let Some(idx) = self.resolve_local(&self.current, &token)? {
            if can_assign && self.match_token(TokenType::Equal)? {
                self.expression()?;
                self.chunk
                    .write_instruction(Instruction::SetLocal(idx), line);
            } else {
                self.chunk
                    .write_instruction(Instruction::GetLocal(idx), line);
            }
        } else {
            let idx = self.identifier_idx(token.lexeme);
            if can_assign && self.match_token(TokenType::Equal)? {
                self.expression()?;
                self.chunk
                    .write_instruction(Instruction::SetGlobal(idx), line);
            } else {
                self.chunk
                    .write_instruction(Instruction::GetGlobal(idx), line);
            }
        }
        Ok(())
    }

    fn resolve_local(&self, compiler: &Compiler, token: &Token) -> Result<Option<usize>> {
        for (idx, local) in compiler.locals.iter().enumerate().rev() {
            if self.identifiers_equal(token, &local.token) {
                if local.depth == -1 {
                    return Err(CompileError::LocalVarInItsOwnInitializer {
                        token: token.to_owned(),
                    });
                }
                return Ok(Some(idx));
            }
        }
        Ok(None)
    }

    fn string(&mut self) -> Result<()> {
        let previous_token = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?;

        let lexeme = &previous_token.lexeme;
        let trimmed_lexeme = &lexeme[1..lexeme.len() - 1];
        let string_value = String::from_utf8_lossy(trimmed_lexeme).to_string();
        let key = if self.heap.intern_table.contains_key(&string_value) {
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

        let constant_idx = self.chunk.add_constant(Value::Object(key));
        self.chunk
            .write_instruction(Instruction::Constant(constant_idx), previous_token.line);

        Ok(())
    }
    fn literal(&mut self) -> Result<()> {
        let previous_token = self
            .previous_token
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
                .previous_token
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

        let prev_variant = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?
            .variant;
        let can_assign = precedence <= Precedence::Assignment;
        self.execute_prefix_parser(prev_variant, can_assign)?;

        while {
            let curr_variant = self
                .current_token
                .as_ref()
                .ok_or(CompileError::MissingCurrentToken)?
                .variant;
            precedence <= self.get_rule_precedence(curr_variant)
        } {
            self.advance()?;

            let prev_variant = self
                .previous_token
                .as_ref()
                .ok_or(CompileError::MissingPreviousToken)?
                .variant;
            self.execute_infix_parser(prev_variant)?;
        }
        if can_assign && self.match_token(TokenType::Equal)? {
            return Err(CompileError::InvalidAssignmentTarget {
                line: self
                    .current_token
                    .as_ref()
                    .ok_or(CompileError::MissingCurrentToken)?
                    .line,
            });
        }
        Ok(())
    }
    fn number(&mut self) -> Result<()> {
        let previous_token = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?;
        let value_str = str::from_utf8(&previous_token.lexeme)
            .map_err(|e| CompileError::InvalidUtf8 { source: e })?;
        let value: f64 = value_str.parse().map_err(|e| CompileError::LiteralParse {
            literal: value_str.to_owned(),
            to: "Double".to_owned(),
            source: e,
        })?;

        let constant_idx = self.chunk.add_constant(Value::Number(value));
        self.chunk
            .write_instruction(Instruction::Constant(constant_idx), previous_token.line);
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
                .previous_token
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
        let line = self
            .previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)?
            .line;
        self.chunk.write_instruction(Instruction::Return, line);
        Ok(())
    }
    pub fn new(source: Vec<u8>, chunk: &'a mut Chunk, heap: &'a mut Heap) -> Self {
        let scanner = Scanner::new(source);
        let compiler = Compiler::new();
        Self {
            previous_token: None,
            current_token: None,
            current: compiler,
            scanner,
            chunk,
            heap,
        }
    }
    fn consume(&mut self, token_variant: TokenType, message: &str) -> Result<()> {
        let token = self
            .current_token
            .to_owned()
            .ok_or(CompileError::MissingCurrentToken)?;
        if token_variant == token.variant {
            self.advance()?;
        } else {
            return Err(CompileError::UnexpectedToken {
                message: message.to_owned(),
                token: token,
            });
        }
        Ok(())
    }
    fn advance(&mut self) -> Result<()> {
        self.previous_token = self.current_token.clone();
        self.current_token = Some(self.scanner.scan_token()?);
        Ok(())
    }
}
