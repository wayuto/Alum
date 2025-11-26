use crate::lexer::Lexer;

pub mod ast;
pub mod error;
pub mod lexer;
pub mod token;

fn main() {
    let src = "
    ";
    let mut lexer = Lexer::new(src.to_string());
    lexer.next_token();
    println!("{:?}", lexer.tok);
}
