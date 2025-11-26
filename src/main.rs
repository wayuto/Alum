use crate::{lexer::Lexer, token::TokenType};

pub mod ast;
pub mod error;
pub mod lexer;
pub mod token;

fn main() {
    let src = "let x = 1";
    let mut lexer = Lexer::new(src);
    loop {
        lexer.next_token();
        let tok = lexer.current_token();
        if tok.token == TokenType::EOF {
            break;
        }
        println!("{:?}", tok);
    }
}
