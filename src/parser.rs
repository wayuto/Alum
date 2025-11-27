use crate::{
    ast::{Exit, Expr, FuncCall, Label, Program, UnaryOp, Val, Var, VarMod},
    lexer::Lexer,
    token::{Literal, Token, TokenType},
};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn parse() -> Program {
        Program { body: Vec::new() }
    }
    pub fn ctrl(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn stmt(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn expr(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn logical(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn comparison(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn additive(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn term(&mut self) -> Expr {
        Expr::Exit(Exit {
            code: Some(Box::new(Expr::Val(Val {
                value: Literal::Number(1f64),
            }))),
        })
    }
    pub fn factor(&mut self) -> Expr {
        if self.lexer.current_token().token == TokenType::LITERAL {
            if let Some(val) = self.lexer.current_token().value {
                self.lexer.next_token();
                return Expr::Val(Val { value: val });
            }
        } else if self.lexer.current_token().token == TokenType::IDENT {
            let name = match self.lexer.current_token().value.unwrap() {
                Literal::Str(s) => s,
                _ => {
                    panic!(
                        "Invalid label name: {:?}",
                        self.lexer.current_token().value.unwrap()
                    )
                }
            };
            self.lexer.next_token();
            match (self.lexer.current_token().token) {
                TokenType::COLON => {
                    self.lexer.next_token();
                    return Expr::Label(Label { name: name });
                }
                TokenType::INC => {
                    self.lexer.next_token();
                    return Expr::UnaryOp(UnaryOp {
                        argument: Box::new(Expr::Var(Var { name: name })),
                        operator: TokenType::INC,
                    });
                }
                TokenType::DEC => {
                    return Expr::UnaryOp(UnaryOp {
                        argument: Box::new(Expr::Var(Var { name: name })),
                        operator: TokenType::DEC,
                    });
                }
                TokenType::LPAREN => {
                    self.lexer.next_token();
                    let mut args: Vec<Expr> = Vec::new();
                    while self.lexer.current_token().token != TokenType::RPAREN {
                        args.push(self.expr());
                    }
                    self.lexer.next_token();
                    return Expr::FuncCall(FuncCall { name, args: args });
                }
                TokenType::EQ => {
                    self.lexer.next_token();
                    let val = self.expr();
                    return Expr::VarMod(Box::new(VarMod { name, value: val }));
                }
                _ => return Expr::Var(Var { name }),
            }
        }
        panic!(
            "Parser: Unexpected token: '{:?}'",
            self.lexer.current_token().token
        )
    }
}
