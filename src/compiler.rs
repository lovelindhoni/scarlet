use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::CompileError;
use crate::heap::{FunctionType, Heap, HeapKey, Upvalue};
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
            Primary => Primary,
        }
    }
}

pub fn compile(source: Vec<u8>, heap: &mut Heap) -> Result<HeapKey> {
    let compiler = Compiler::new(heap.allocate_function(None), FunctionType::Script);
    let mut parser = Parser::new(source, compiler, heap);
    parser.advance()?;
    while !parser.match_token(TokenType::Eof)? {
        parser.declaration()?;
    }
    let (function, _upvalues) = parser.end_compiler()?;
    Ok(function)
}

struct Local {
    token: Token,
    depth: i64,
    is_captured: bool,
}

impl Local {
    pub fn new(token: Token, depth: i64) -> Self {
        Self {
            token,
            depth,
            is_captured: false,
        }
    }
}

struct Compiler {
    pub function_type: FunctionType,
    pub function: HeapKey,
    pub locals: Vec<Local>,
    pub scope_depth: i64,
    pub upvalues: Vec<Upvalue>,
}

impl Compiler {
    pub fn new(function_key: HeapKey, function_type: FunctionType) -> Self {
        let local = match function_type {
            FunctionType::Method | FunctionType::Initializer => Local::new(
                Token {
                    variant: TokenType::This,
                    lexeme: b"this".to_vec(),
                    line: 0,
                },
                0,
            ),
            _ => Local::new(
                Token {
                    variant: TokenType::Nil,
                    lexeme: Vec::new(),
                    line: 0,
                },
                0,
            ),
        };
        Compiler {
            function: function_key,
            function_type,
            locals: vec![local],
            scope_depth: 0,
            upvalues: Vec::new(),
        }
    }
}

struct ClassCompiler {
    pub has_super_class: bool,
}

struct Parser<'a> {
    previous_token: Option<Token>,
    current_token: Option<Token>,
    scanner: Scanner,
    compilers: Vec<Compiler>,
    class_compilers: Vec<ClassCompiler>,
    heap: &'a mut Heap,
}

impl<'a> Parser<'a> {
    fn prev_token(&self) -> Result<&Token> {
        self.previous_token
            .as_ref()
            .ok_or(CompileError::MissingPreviousToken)
    }

    fn curr_token(&self) -> Result<&Token> {
        self.current_token
            .as_ref()
            .ok_or(CompileError::MissingCurrentToken)
    }

    fn prev_line(&self) -> Result<u64> {
        Ok(self.prev_token()?.line)
    }

    fn prev_lexeme(&self) -> Result<Vec<u8>> {
        Ok(self.prev_token()?.lexeme.clone())
    }

    fn emit(&mut self, instruction: Instruction) -> Result<()> {
        let line = self.prev_line()?;
        self.current_chunk().write_instruction(instruction, line);
        Ok(())
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        let function = self.current_compiler().function;
        &mut self.heap.get_mut_function(function).chunk
    }

    fn identifier_constant(&mut self, lexeme: Vec<u8>) -> usize {
        let identifier = String::from_utf8_lossy(&lexeme).to_string();
        let key = self.heap.allocate_or_intern_string(&identifier);
        self.current_chunk().add_constant(Value::Object(key))
    }

    fn identifier_constant_from_prev(&mut self) -> Result<usize> {
        let lexeme = self.prev_lexeme()?;
        Ok(self.identifier_constant(lexeme))
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
        Ok(self.curr_token()?.variant == token_variant)
    }

    pub fn declaration(&mut self) -> Result<()> {
        if self.match_token(TokenType::Class)? {
            self.class_declaration()?;
        } else if self.match_token(TokenType::Fun)? {
            self.fun_declaration()?;
        } else if self.match_token(TokenType::Let)? {
            self.let_declaration()?;
        } else {
            self.statement()?;
        }
        Ok(())
    }

    fn class_declaration(&mut self) -> Result<()> {
        self.consume(TokenType::Identifier, "Expect class name")?;
        let class_name = self.prev_token()?.to_owned();
        let name_constant = self.identifier_constant_from_prev()?;
        self.declare_variable()?;

        self.emit(Instruction::Class(name_constant))?;
        self.define_variable(name_constant)?;

        self.class_compilers.push(ClassCompiler {
            has_super_class: false,
        });

        if self.match_token(TokenType::Inherits)? {
            self.consume(
                TokenType::Identifier,
                "Expect superclass name after inherits keyword in class",
            )?;
            self.variable(false)?;
            if self.identifiers_equal(&class_name, self.prev_token()?) {
                return Err(CompileError::Selfheritance);
            }
            self.begin_scope();
            self.add_local(self.synthetic_token("super"));
            self.define_variable(0)?;
            self.named_variable(class_name.clone(), false)?;
            self.emit(Instruction::Inherit)?;
            self.class_compilers.last_mut().unwrap().has_super_class = true;
        }

        self.named_variable(class_name, false)?;
        self.consume(TokenType::LeftBrace, "Expect '{' before class body")?;

        while !self.check(TokenType::RightBrace)? && !self.check(TokenType::Eof)? {
            self.method()?;
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body")?;
        self.emit(Instruction::Pop)?;

        let had_superclass = self.class_compilers.last().unwrap().has_super_class;
        self.class_compilers.pop();

        if had_superclass {
            self.end_scope()?;
        }
        Ok(())
    }

    fn method(&mut self) -> Result<()> {
        self.consume(TokenType::Identifier, "Expect method name")?;
        let constant = self.identifier_constant_from_prev()?;
        let function_type = if str::from_utf8(&self.prev_lexeme()?)
            .map_err(|e| CompileError::InvalidUtf8 { source: e })?
            == "init"
        {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };
        self.function(function_type)?;
        self.emit(Instruction::Method(constant))?;
        Ok(())
    }

    fn fun_declaration(&mut self) -> Result<()> {
        let function_name = self.parse_variable("Expect function name")?;
        self.mark_intialized()?;
        self.function(FunctionType::Function)?;
        self.define_variable(function_name)?;
        Ok(())
    }

    fn function(&mut self, function_type: FunctionType) -> Result<()> {
        let function_name = if let FunctionType::Function = function_type {
            Some(String::from_utf8_lossy(&self.prev_lexeme()?).to_string())
        } else {
            None
        };
        let compiler = Compiler::new(self.heap.allocate_function(function_name), function_type);
        self.compilers.push(compiler);
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after function name")?;
        if !self.check(TokenType::RightParen)? {
            loop {
                self.heap
                    .get_mut_function(self.current_compiler().function)
                    .arity += 1;
                let parameter = self.parse_variable("Expect parameter name")?;
                self.define_variable(parameter)?;
                if !self.match_token(TokenType::Comma)? {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters")?;
        self.consume(TokenType::LeftBrace, "Expect '{' before function body")?;
        self.block()?;
        let (function, upvalues) = self.end_compiler()?;
        let idx = self.current_chunk().add_constant(Value::Object(function));
        self.emit(Instruction::Closure(idx, upvalues.into_boxed_slice()))?;
        Ok(())
    }

    fn return_statement(&mut self) -> Result<()> {
        if let FunctionType::Script = self.current_compiler().function_type {
            return Err(CompileError::ReturnFromTopLevel {
                token: self.prev_token()?.to_owned(),
            });
        }
        if self.match_token(TokenType::Semicolon)? {
            self.emit_return()?;
        } else {
            if let FunctionType::Initializer = self.current_compiler().function_type {
                return Err(CompileError::ReturnFromClassInitializer {
                    token: self.prev_token()?.to_owned(),
                });
            }
            self.expression()?;
            self.consume(TokenType::Semicolon, "Expect ';' after return value")?;
            self.emit(Instruction::Return)?;
        }
        Ok(())
    }

    fn parse_variable(&mut self, message: &str) -> Result<usize> {
        self.consume(TokenType::Identifier, message)?;
        self.declare_variable()?;
        if self.current_compiler().scope_depth > 0 {
            Ok(0)
        } else {
            self.identifier_constant_from_prev()
        }
    }

    fn declare_variable(&mut self) -> Result<()> {
        if self.current_compiler().scope_depth > 0 {
            let token = self
                .previous_token
                .to_owned()
                .ok_or(CompileError::MissingPreviousToken)?;
            for local in self.current_compiler().locals.iter().rev() {
                if local.depth != -1 && local.depth < self.current_compiler().scope_depth {
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
        self.compilers
            .last_mut()
            .unwrap()
            .locals
            .push(Local::new(token, -1));
    }

    fn define_variable(&mut self, global: usize) -> Result<()> {
        if self.current_compiler().scope_depth == 0 {
            self.emit(Instruction::DefineGlobal(global))?;
        } else {
            self.mark_intialized()?;
        }
        Ok(())
    }

    fn mark_intialized(&mut self) -> Result<()> {
        if self.current_compiler().scope_depth != 0 {
            self.compilers
                .last_mut()
                .unwrap()
                .locals
                .last_mut()
                .ok_or(CompileError::LocalsEmpty)?
                .depth = self.current_compiler().scope_depth;
        }
        Ok(())
    }

    fn let_declaration(&mut self) -> Result<()> {
        let variable = self.parse_variable("Expect variable name")?;
        if self.match_token(TokenType::Equal)? {
            self.expression()?;
        } else {
            self.emit(Instruction::Nil)?;
        }
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration",
        )?;
        self.define_variable(variable)?;
        Ok(())
    }

    fn statement(&mut self) -> Result<()> {
        if self.match_token(TokenType::If)? {
            self.if_statement()?;
        } else if self.match_token(TokenType::Return)? {
            self.return_statement()?;
        } else if self.match_token(TokenType::While)? {
            self.while_statement()?;
        } else if self.match_token(TokenType::For)? {
            self.for_statement()?;
        } else if self.match_token(TokenType::LeftBrace)? {
            self.begin_scope();
            self.block()?;
            self.end_scope()?;
        } else {
            self.expression_statement()?;
        }
        Ok(())
    }

    fn for_statement(&mut self) -> Result<()> {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;
        if self.match_token(TokenType::Semicolon)? {
            // No initializer.
        } else if self.match_token(TokenType::Let)? {
            self.let_declaration()?;
        } else {
            self.expression_statement()?;
        }

        let mut loop_start = self.current_chunk().instructions.len();
        let mut exit_jump: Option<usize> = None;

        if !self.match_token(TokenType::Semicolon)? {
            self.expression()?;
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;
            self.emit(Instruction::JumpIfFalse(usize::MAX))?;
            exit_jump = Some(self.current_chunk().instructions.len() - 1);
            self.emit(Instruction::Pop)?;
        }

        if !self.match_token(TokenType::RightParen)? {
            self.emit(Instruction::Jump(usize::MAX))?;
            let body_jump = self.current_chunk().instructions.len() - 1;
            let increment_start = self.current_chunk().instructions.len();

            self.expression()?;
            self.emit(Instruction::Pop)?;
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

            self.emit_loop(loop_start)?;
            loop_start = increment_start;
            self.patch_jump(body_jump)?;
        }

        self.statement()?;
        self.emit_loop(loop_start)?;

        if let Some(jump) = exit_jump {
            self.patch_jump(jump)?;
            self.emit(Instruction::Pop)?;
        }

        self.end_scope()?;
        Ok(())
    }

    fn while_statement(&mut self) -> Result<()> {
        let loop_start = self.current_chunk().instructions.len();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'")?;
        self.expression()?;
        self.consume(
            TokenType::RightParen,
            "Expect ')' after condition in 'while'",
        )?;
        self.emit(Instruction::JumpIfFalse(usize::MAX))?;
        let exit_jump = self.current_chunk().instructions.len() - 1;
        self.emit(Instruction::Pop)?;
        self.statement()?;
        self.emit_loop(loop_start)?;
        self.patch_jump(exit_jump)?;
        self.emit(Instruction::Pop)?;
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result<()> {
        let offset = self.current_chunk().instructions.len() - loop_start + 1;
        self.emit(Instruction::Loop(offset))?;
        Ok(())
    }

    fn if_statement(&mut self) -> Result<()> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'")?;
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition in 'if'")?;

        self.emit(Instruction::JumpIfFalse(usize::MAX))?;
        let then_jump = self.current_chunk().instructions.len() - 1;
        self.emit(Instruction::Pop)?;
        self.statement()?;

        self.emit(Instruction::Jump(usize::MAX))?;
        let else_jump = self.current_chunk().instructions.len() - 1;
        self.patch_jump(then_jump)?;
        self.emit(Instruction::Pop)?;

        if self.match_token(TokenType::Else)? {
            self.statement()?;
        }
        self.patch_jump(else_jump)?;
        Ok(())
    }

    fn patch_jump(&mut self, jump_index: usize) -> Result<()> {
        let current = self.current_chunk().instructions.len();
        let jump = current
            .checked_sub(jump_index + 1)
            .ok_or(CompileError::InvalidJumpPatch { index: jump_index })?;
        match &mut self.current_chunk().instructions[jump_index] {
            Instruction::JumpIfFalse(offset) | Instruction::Jump(offset) => *offset = jump,
            _ => return Err(CompileError::InvalidJumpPatch { index: jump_index }),
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
        self.compilers.last_mut().unwrap().scope_depth += 1;
    }

    fn end_scope(&mut self) -> Result<()> {
        self.compilers.last_mut().unwrap().scope_depth -= 1;
        while let Some(local) = self.current_compiler().locals.last() {
            if local.depth <= self.current_compiler().scope_depth {
                break;
            }
            let instruction = if self.current_compiler().locals.last().unwrap().is_captured {
                Instruction::CloseUpvalue
            } else {
                Instruction::Pop
            };
            self.emit(instruction)?;
            self.compilers.last_mut().unwrap().locals.pop();
        }
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression")?;
        self.emit(Instruction::Pop)?;
        Ok(())
    }

    fn get_rule_precedence(&self, token_variant: TokenType) -> Precedence {
        match token_variant {
            TokenType::Minus | TokenType::Plus => Precedence::Term,
            TokenType::Slash | TokenType::Star | TokenType::Modulo => Precedence::Factor,
            TokenType::BangEqual | TokenType::EqualEqual => Precedence::Equality,
            TokenType::And => Precedence::And,
            TokenType::Or => Precedence::Or,
            TokenType::LeftParen | TokenType::Dot => Precedence::Call,
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
            TokenType::Super => self._super(),
            TokenType::This => self.this(),
            _ => Err(CompileError::MissingPrefixParser {
                message: "Expect expression".to_owned(),
                token: self.curr_token()?.clone(),
            }),
        }
    }

    fn this(&mut self) -> Result<()> {
        if self.class_compilers.is_empty() {
            return Err(CompileError::ThisOutsideClass {
                token: self.prev_token()?.to_owned(),
            });
        }
        self.variable(false)
    }

    fn synthetic_token(&self, text: &str) -> Token {
        Token {
            variant: TokenType::Identifier,
            lexeme: text.as_bytes().to_vec(),
            line: 0,
        }
    }

    fn _super(&mut self) -> Result<()> {
        if self.class_compilers.is_empty() {
            return Err(CompileError::SuperOutsideClass);
        } else if !self.class_compilers.last().unwrap().has_super_class {
            return Err(CompileError::SuperInBaseClass);
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'")?;
        self.consume(TokenType::Identifier, "Expect superclass method name.")?;
        let name = self.identifier_constant_from_prev()?;

        self.named_variable(self.synthetic_token("this"), false)?;
        if self.match_token(TokenType::LeftParen)? {
            let arg_count = self.argument_list()?;
            self.named_variable(self.synthetic_token("super"), false)?;
            self.emit(Instruction::SuperInvoke(name, arg_count))?;
        } else {
            self.named_variable(self.synthetic_token("super"), false)?;
            self.emit(Instruction::GetSuper(name))?;
        }
        Ok(())
    }

    fn execute_infix_parser(&mut self, token_variant: TokenType, can_assign: bool) -> Result<()> {
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
            TokenType::And => self.and(),
            TokenType::Or => self.or(),
            TokenType::LeftParen => self.call(),
            TokenType::Dot => self.dot(can_assign),
            _ => Err(CompileError::MissingInfixParser(self.prev_token()?.variant)),
        }
    }

    fn dot(&mut self, can_assign: bool) -> Result<()> {
        self.consume(
            TokenType::Identifier,
            "Expect property name after '.' in an instance",
        )?;
        let name = self.identifier_constant_from_prev()?;
        if can_assign && self.match_token(TokenType::Equal)? {
            self.expression()?;
            self.emit(Instruction::SetProperty(name))?;
        } else if self.match_token(TokenType::LeftParen)? {
            let arg_count = self.argument_list()?;
            self.emit(Instruction::Invoke(name, arg_count))?;
        } else {
            self.emit(Instruction::GetProperty(name))?;
        }
        Ok(())
    }

    fn call(&mut self) -> Result<()> {
        let arg_count = self.argument_list()?;
        self.emit(Instruction::Call(arg_count))?;
        Ok(())
    }

    fn argument_list(&mut self) -> Result<usize> {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen)? {
            loop {
                self.expression()?;
                arg_count += 1;
                if !self.match_token(TokenType::Comma)? {
                    break;
                }
            }
        }
        self.consume(
            TokenType::RightParen,
            "Expect ')' after arguments to an function",
        )?;
        Ok(arg_count)
    }

    fn and(&mut self) -> Result<()> {
        self.emit(Instruction::JumpIfFalse(usize::MAX))?;
        let end_jump = self.current_chunk().instructions.len() - 1;
        self.emit(Instruction::Pop)?;
        self.parse_precedence(Precedence::And)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn or(&mut self) -> Result<()> {
        self.emit(Instruction::JumpIfFalse(usize::MAX))?;
        let else_jump = self.current_chunk().instructions.len() - 1;
        self.emit(Instruction::Jump(usize::MAX))?;
        let end_jump = self.current_chunk().instructions.len() - 1;
        self.patch_jump(else_jump)?;
        self.emit(Instruction::Pop)?;
        self.parse_precedence(Precedence::Or)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn variable(&mut self, can_assign: bool) -> Result<()> {
        let previous = self
            .previous_token
            .to_owned()
            .ok_or(CompileError::MissingPreviousToken)?;
        self.named_variable(previous, can_assign)
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) -> Result<()> {
        let line = token.line;
        if let Some(idx) = self.resolve_local(self.current_compiler(), &token)? {
            if can_assign && self.match_token(TokenType::Equal)? {
                self.expression()?;
                self.current_chunk()
                    .write_instruction(Instruction::SetLocal(idx), line);
            } else {
                self.current_chunk()
                    .write_instruction(Instruction::GetLocal(idx), line);
            }
        } else if let Some(idx) = self.resolve_upvalue(self.compilers.len() - 1, &token)? {
            if can_assign && self.match_token(TokenType::Equal)? {
                self.expression()?;
                self.current_chunk()
                    .write_instruction(Instruction::SetUpvalue(idx), line);
            } else {
                self.current_chunk()
                    .write_instruction(Instruction::GetUpvalue(idx), line);
            }
        } else {
            let idx = self.identifier_constant(token.lexeme);
            if can_assign && self.match_token(TokenType::Equal)? {
                self.expression()?;
                self.current_chunk()
                    .write_instruction(Instruction::SetGlobal(idx), line);
            } else {
                self.current_chunk()
                    .write_instruction(Instruction::GetGlobal(idx), line);
            }
        }
        Ok(())
    }

    fn resolve_upvalue(&mut self, idx: usize, token: &Token) -> Result<Option<usize>> {
        if idx == 0 {
            return Ok(None);
        }
        let enclosing = &self.compilers[idx - 1];
        if let Some(local) = self.resolve_local(enclosing, token)? {
            let enclosing = &mut self.compilers[idx - 1];
            enclosing.locals[local].is_captured = true;
            return Ok(Some(self.add_upvalue(idx, local, true)));
        }
        if let Some(upvalue) = self.resolve_upvalue(idx - 1, token)? {
            return Ok(Some(self.add_upvalue(idx, upvalue, false)));
        }
        Ok(None)
    }

    fn add_upvalue(&mut self, compiler_idx: usize, local_idx: usize, is_local: bool) -> usize {
        let compiler = &mut self.compilers[compiler_idx];
        let upvalue_count = compiler.upvalues.len();
        for (i, upvalue) in compiler.upvalues.iter().enumerate() {
            if upvalue.index == local_idx && upvalue.is_local == is_local {
                return i;
            }
        }
        compiler.upvalues.push(Upvalue {
            is_local,
            index: local_idx,
        });
        upvalue_count
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
        let lexeme = self.prev_lexeme()?;
        let line = self.prev_line()?;
        let trimmed = &lexeme[1..lexeme.len() - 1];
        let string_value = String::from_utf8_lossy(trimmed).to_string();
        let key = self.heap.allocate_or_intern_string(&string_value);
        let constant_idx = self.current_chunk().add_constant(Value::Object(key));
        self.current_chunk()
            .write_instruction(Instruction::Constant(constant_idx), line);
        Ok(())
    }

    fn literal(&mut self) -> Result<()> {
        let instruction = match self.prev_token()?.variant {
            TokenType::True => Instruction::True,
            TokenType::False => Instruction::False,
            TokenType::Nil => Instruction::Nil,
            _ => return Ok(()), // unreachable
        };
        self.emit(instruction)
    }

    fn binary(&mut self) -> Result<()> {
        let variant = self.prev_token()?.variant;
        let rule = self.get_rule_precedence(variant);
        self.parse_precedence(rule.next())?;
        match variant {
            TokenType::Plus => self.emit(Instruction::Add)?,
            TokenType::Minus => self.emit(Instruction::Subtract)?,
            TokenType::Star => self.emit(Instruction::Multiply)?,
            TokenType::Slash => self.emit(Instruction::Divide)?,
            TokenType::Modulo => self.emit(Instruction::Modulo)?,
            TokenType::EqualEqual => self.emit(Instruction::Equal)?,
            TokenType::BangEqual => {
                self.emit(Instruction::Equal)?;
                self.emit(Instruction::Not)?;
            }
            TokenType::Greater => self.emit(Instruction::Greater)?,
            TokenType::GreaterEqual => {
                self.emit(Instruction::Less)?;
                self.emit(Instruction::Not)?;
            }
            TokenType::Less => self.emit(Instruction::Less)?,
            TokenType::LessEqual => {
                self.emit(Instruction::Greater)?;
                self.emit(Instruction::Not)?;
            }
            _ => {} // unreachable
        }
        Ok(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.advance()?;
        let prev_variant = self.prev_token()?.variant;
        let can_assign = precedence <= Precedence::Assignment;
        self.execute_prefix_parser(prev_variant, can_assign)?;

        while {
            let curr_variant = self.curr_token()?.variant;
            precedence <= self.get_rule_precedence(curr_variant)
        } {
            self.advance()?;
            let prev_variant = self.prev_token()?.variant;
            self.execute_infix_parser(prev_variant, can_assign)?;
        }

        if can_assign && self.match_token(TokenType::Equal)? {
            return Err(CompileError::InvalidAssignmentTarget {
                line: self.curr_token()?.line,
            });
        }
        Ok(())
    }

    fn number(&mut self) -> Result<()> {
        let (lexeme, line) = (self.prev_lexeme()?, self.prev_line()?);
        let value_str =
            str::from_utf8(&lexeme).map_err(|e| CompileError::InvalidUtf8 { source: e })?;
        let value: f64 = value_str.parse().map_err(|e| CompileError::LiteralParse {
            literal: value_str.to_owned(),
            to: "Double".to_owned(),
            source: e,
        })?;
        let constant_idx = self.current_chunk().add_constant(Value::Number(value));
        self.current_chunk()
            .write_instruction(Instruction::Constant(constant_idx), line);
        Ok(())
    }

    fn grouping(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ) after expression")?;
        Ok(())
    }

    fn unary(&mut self) -> Result<()> {
        let variant = self.prev_token()?.variant;
        self.parse_precedence(Precedence::Unary)?;
        match variant {
            TokenType::Minus => self.emit(Instruction::Negate)?,
            TokenType::Bang => self.emit(Instruction::Not)?,
            _ => {}
        }
        Ok(())
    }

    fn expression(&mut self) -> Result<()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn end_compiler(&mut self) -> Result<(HeapKey, Vec<Upvalue>)> {
        self.emit_return()?;
        let compiler = self.compilers.pop().unwrap();
        Ok((compiler.function, compiler.upvalues))
    }

    fn emit_return(&mut self) -> Result<()> {
        if let FunctionType::Initializer = self.current_compiler().function_type {
            self.emit(Instruction::GetLocal(0))?;
        } else {
            self.emit(Instruction::Nil)?;
        }
        self.emit(Instruction::Return)
    }

    pub fn new(source: Vec<u8>, compiler: Compiler, heap: &'a mut Heap) -> Self {
        let scanner = Scanner::new(source);
        let compilers = vec![compiler];
        Self {
            previous_token: None,
            current_token: None,
            scanner,
            heap,
            compilers,
            class_compilers: Vec::new(),
        }
    }

    fn current_compiler(&self) -> &Compiler {
        self.compilers.last().unwrap()
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
                token,
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
