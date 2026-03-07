use thiserror::Error;

use crate::scanner::{Token, TokenType};

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("Chunk '{name}' is empty")]
    EmptyChunk { name: String },

    #[error("Instruction pointer {ip} out of bounds (len = {len})")]
    InvalidInstructionPointer { ip: usize, len: usize },
}

#[derive(Debug, Error)]
pub enum HeapError {
    #[error("Internal: Expired arena key")]
    ExpiredArenaKey,

    #[error("Internal: Invalid interned key: expected `{expected}`, found `{found:?}` in arena")]
    InvalidInternedKey { expected: String, found: String },
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("{msg}", msg = scan_error_helper(.message, .line, .lexeme))]
    UnterminatedString {
        message: String,
        line: u64,
        lexeme: Vec<u8>,
    },

    #[error("{msg}", msg = scan_error_helper(.message, .line, .lexeme))]
    UnexpectedCharacter {
        message: String,
        line: u64,
        lexeme: Vec<u8>,
    },
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Syntax Error on {0}")]
    Scan(#[from] ScanError),

    #[error(transparent)]
    Heap(#[from] HeapError),

    #[error("Internal: Missing current token when parsing")]
    MissingCurrentToken,

    #[error("Internal: Missing previous token when parsing")]
    MissingPreviousToken,

    #[error("Internal: Invalid jump patch at instruction index {index}")]
    InvalidJumpPatch { index: usize },

    #[error("{msg}", msg = compile_error_helper(.message, .token))]
    UnexpectedToken { message: String, token: Token },

    #[error("Failed to parse literal as valid UTF-8")]
    InvalidUtf8 { source: std::str::Utf8Error },

    #[error("{msg}", msg= compile_error_helper("Can't return from top level code", token))]
    ReturnFromTopLevel { token: Token },

    #[error("Failed to parse literal {literal} to {to}, source: {source}")]
    LiteralParse {
        literal: String,
        to: String,
        source: std::num::ParseFloatError,
    },

    #[error("Invalid assignment target\n[line {line}]")]
    InvalidAssignmentTarget { line: u64 },

    #[error("Internal: No local variables found in the scope")]
    LocalsEmpty,

    #[error("{msg}", msg= compile_error_helper("Can't read local variable in its own initializer", token))]
    LocalVarInItsOwnInitializer { token: Token },

    #[error("{msg}", msg = compile_error_helper("Already variable with this name in this scope", token))]
    RedefinitionOfLocalVar { token: Token },

    #[error("{msg}", msg = compile_error_helper(.message, .token))]
    MissingPrefixParser { message: String, token: Token },

    #[error("Internal: Missing infix rule for the token variant {0}")]
    MissingInfixParser(TokenType),
}

#[derive(Debug, Error)]
pub enum InterpretError {
    #[error("{msg}",msg = interpet_error_helper("Can only call functions and classes", .line))]
    UncallableObject { line: u64 },

    #[error("{msg}", msg = interpet_error_helper(.message, .line))]
    ArgumentsCountMismatch { message: String, line: u64 },

    #[error("{msg}",msg = interpet_error_helper(.message, .line))]
    TypeError { line: u64, message: String },

    #[error("{msg}",msg = interpet_error_helper(format!("Division By Zero: {}/{}", .left_num, .right_num).as_str(), .line))]
    DivisionByZero {
        line: u64,
        left_num: f64,
        right_num: f64,
    },

    #[error("Invalid binary operation")]
    InvalidBinaryOp,

    #[error("{msg}",msg = interpet_error_helper(format!("Undefined variable '{}'", .identifier).as_str(), .line))]
    UndefinedVariable { identifier: String, line: u64 },
}

fn interpet_error_helper(message: &str, line: &u64) -> String {
    format!("{}\n[line {}]", message, line)
}

fn compile_error_helper(message: &str, token: &Token) -> String {
    format!(
        "[line {}] at '{}': {}",
        token.line,
        if token.variant == TokenType::Eof {
            String::from("the end")
        } else {
            String::from_utf8_lossy(&token.lexeme).to_string()
        },
        message
    )
}

fn scan_error_helper(message: &str, line: &u64, lexeme: &[u8]) -> String {
    format!(
        "[line {}] at '{}': {}",
        line,
        String::from_utf8_lossy(lexeme),
        message
    )
}
