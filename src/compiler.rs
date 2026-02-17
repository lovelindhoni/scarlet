use crate::scanner::{Scanner, TokenType};

pub fn compile(source: &[u8]) {
    let mut scanner = Scanner::new(source);
    loop {
        let token = scanner.scan_token();
        match token.variant {
            TokenType::UnterminatedString => {
                print!("Unterminated String ");
                print!("Line: {} ", token.line);
                println!(
                    "{}",
                    str::from_utf8(&source[token.start..token.start + token.length]).unwrap()
                )
            }
            TokenType::UnexpectedCharacter => {
                print!("Unexpected Character");
                print!("Line: {} ", token.line);
                println!(
                    "{}",
                    str::from_utf8(&source[token.start..token.start + token.length]).unwrap()
                )
            }
            TokenType::Eof => return,
            _ => {
                print!("Token Type: {:?} ", token.variant);
                print!("Line: {} ", token.line);
                println!(
                    "{}",
                    str::from_utf8(&source[token.start..token.start + token.length]).unwrap()
                );
            }
        }
    }
}
