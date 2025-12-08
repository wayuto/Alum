use crate::{
    ast::{
        BinOp, Exit, Expr, Extern, FuncCall, FuncDecl, Goto, If, In, Label, Out, Program, Return,
        Stmt, UnaryOp, Val, Var, VarDecl, VarMod, While,
    },
    lexer::Lexer,
    token::{Literal, TokenType},
};

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self { lexer }
    }

    pub fn parse(&mut self) -> Program {
        self.lexer.next_token();
        let mut exprs: Vec<Expr> = Vec::new();
        while self.lexer.current_token().token != TokenType::EOF {
            exprs.push(self.ctrl());
        }
        Program { body: exprs }
    }
    fn ctrl(&mut self) -> Expr {
        match self.lexer.current_token().token {
            TokenType::IF => {
                self.lexer.next_token();
                let cond = self.expr();
                let body = self.stmt();
                if self.lexer.current_token().token == TokenType::ELSE {
                    self.lexer.next_token();
                    let else_body = self.stmt();
                    match cond.clone() {
                        Expr::Val(val) => match val.value {
                            Literal::Bool(b) => {
                                if b {
                                    return body;
                                } else {
                                    return else_body;
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    return Expr::If(If {
                        condition: Box::new(cond),
                        then: Box::new(body),
                        else_branch: Some(Box::new(else_body)),
                    });
                }
                match cond.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(b) => {
                            if b {
                                return body;
                            } else {
                                return Expr::Stmt(Stmt { body: vec![] });
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Expr::If(If {
                    condition: Box::new(cond),
                    then: Box::new(body),
                    else_branch: None,
                })
            }
            TokenType::WHILE => {
                self.lexer.next_token();
                let cond = self.expr();
                let body = self.stmt();
                match cond.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(b) => {
                            if b {
                                let lbl = format!("loop{:p}", &body);
                                return Expr::Stmt(Stmt {
                                    body: vec![
                                        Expr::Label(Label { name: lbl.clone() }),
                                        body,
                                        Expr::Goto(Goto { label: lbl }),
                                    ],
                                });
                            } else {
                                return Expr::Stmt(Stmt { body: vec![] });
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Expr::While(While {
                    condition: Box::new(cond),
                    body: Box::new(body),
                })
            }
            TokenType::FUNCDECL => {
                self.lexer.next_token();
                let name = self.get_ident();
                let mut params: Vec<String> = Vec::new();
                self.lexer.next_token();
                if self.lexer.current_token().token != TokenType::LPAREN {
                    panic!("Parser: Expected: '('")
                }
                while self.lexer.current_token().token != TokenType::RPAREN {
                    if self.lexer.current_token().token == TokenType::IDENT {
                        params.push(self.get_ident())
                    }
                    self.lexer.next_token();
                }
                self.lexer.next_token();
                let body = self.stmt();
                Expr::FuncDecl(FuncDecl {
                    name,
                    params,
                    body: Box::new(body),
                })
            }
            _ => self.stmt(),
        }
    }
    fn stmt(&mut self) -> Expr {
        if self.lexer.current_token().token == TokenType::LBRACE {
            let mut exprs: Vec<Expr> = Vec::new();
            self.lexer.next_token();

            while self.lexer.current_token().token != TokenType::RBRACE {
                if self.lexer.current_token().token == TokenType::EOF {
                    panic!("Parser: Expected: '}}'")
                }
                exprs.push(self.ctrl());
            }

            self.lexer.next_token();
            return Expr::Stmt(Stmt { body: exprs });
        }
        if self.lexer.current_token().token == TokenType::IF
            || self.lexer.current_token().token == TokenType::WHILE
            || self.lexer.current_token().token == TokenType::FUNCDECL
        {
            return self.ctrl();
        }
        self.expr()
    }
    fn expr(&mut self) -> Expr {
        match self.lexer.current_token().token {
            TokenType::EXIT => {
                self.lexer.next_token();
                let status = self.expr();
                Expr::Exit(Exit {
                    code: Box::new(status),
                })
            }
            TokenType::GOTO => {
                self.lexer.next_token();
                let name = self.get_ident();
                self.lexer.next_token();
                self.lexer.next_token();
                Expr::Goto(Goto { label: name })
            }
            TokenType::VARDECL => {
                self.lexer.next_token();
                let name = self.get_ident();
                self.lexer.next_token();
                if self.lexer.current_token().token != TokenType::EQ {
                    panic!("Parser: Expected: '='")
                }
                self.lexer.next_token();
                let value = self.expr();
                Expr::VarDecl(VarDecl {
                    name,
                    value: Box::new(value),
                })
            }
            TokenType::OUT => {
                self.lexer.next_token();
                let value = self.expr();
                Expr::Out(Out {
                    value: Box::new(value),
                })
            }
            TokenType::IN => {
                self.lexer.next_token();
                let name = self.get_ident();
                Expr::In(In { name })
            }
            TokenType::RETURN => {
                self.lexer.next_token();
                let value = self.expr();
                Expr::Return(Return {
                    value: Some(Box::new(value)),
                })
            }
            TokenType::EXTERN => {
                self.lexer.next_token();
                let func = self.get_ident();
                self.lexer.next_token();
                Expr::Extern(Extern { func })
            }
            TokenType::IF => self.ctrl(),
            TokenType::WHILE => self.ctrl(),
            TokenType::LBRACE => self.ctrl(),
            _ => self.logical(),
        }
    }
    fn logical(&mut self) -> Expr {
        let mut left = self.comparison();
        while self.lexer.current_token().token == TokenType::LOGAND
            || self.lexer.current_token().token == TokenType::LOGOR
            || self.lexer.current_token().token == TokenType::LOGXOR
        {
            let op = self.lexer.current_token().token;
            self.lexer.next_token();
            let right = self.comparison();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Expr::Val(Val {
                                value: Literal::Number(n & m),
                            });
                        }
                        TokenType::LOGOR => {
                            return Expr::Val(Val {
                                value: Literal::Number(n | m),
                            });
                        }
                        TokenType::LOGXOR => {
                            return Expr::Val(Val {
                                value: Literal::Number(n ^ m),
                            });
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n & m),
                            });
                        }
                        TokenType::LOGOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n | m),
                            });
                        }
                        TokenType::LOGXOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n ^ m),
                            });
                        }
                        _ => {}
                    },
                    (_, _) => {}
                },
                (_, _) => {}
            }
            left = Expr::BinOp(BinOp {
                left: Box::new(left),
                right: Box::new(right),
                operator: op,
            })
        }
        left
    }
    fn comparison(&mut self) -> Expr {
        let mut left = self.additive();
        while self.lexer.current_token().token == TokenType::COMPEQ
            || self.lexer.current_token().token == TokenType::COMPNE
            || self.lexer.current_token().token == TokenType::COMPLT
            || self.lexer.current_token().token == TokenType::COMPLE
            || self.lexer.current_token().token == TokenType::COMPGT
            || self.lexer.current_token().token == TokenType::COMPGE
            || self.lexer.current_token().token == TokenType::COMPAND
            || self.lexer.current_token().token == TokenType::COMPOR
        {
            let op = self.lexer.current_token().token;
            self.lexer.next_token();
            let right = self.additive();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::COMPEQ => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n == m),
                            });
                        }
                        TokenType::COMPNE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n != m),
                            });
                        }
                        TokenType::COMPGT => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n > m),
                            });
                        }
                        TokenType::COMPGE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n >= m),
                            });
                        }
                        TokenType::COMPLT => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n < m),
                            });
                        }
                        TokenType::COMPLE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n <= m),
                            });
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::COMPAND => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n && m),
                            });
                        }
                        TokenType::COMPOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n || m),
                            });
                        }
                        _ => {}
                    },
                    (_, _) => {}
                },
                (_, _) => {}
            }
            left = Expr::BinOp(BinOp {
                left: Box::new(left),
                right: Box::new(right),
                operator: op,
            });
        }
        return left;
    }
    fn additive(&mut self) -> Expr {
        let mut left = self.term();
        while self.lexer.current_token().token == TokenType::ADD
            || self.lexer.current_token().token == TokenType::SUB
        {
            let op = self.lexer.current_token().token;
            self.lexer.next_token();
            let right = self.term();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::ADD => {
                            return Expr::Val(Val {
                                value: Literal::Number(n + m),
                            });
                        }
                        TokenType::SUB => {
                            return Expr::Val(Val {
                                value: Literal::Number(n - m),
                            });
                        }
                        _ => {}
                    },
                    (_, _) => {}
                },
                (_, _) => {}
            }
            left = Expr::BinOp(BinOp {
                left: Box::new(left),
                right: Box::new(right),
                operator: op,
            });
        }
        return left;
    }
    fn term(&mut self) -> Expr {
        let mut left = self.factor();
        while self.lexer.current_token().token == TokenType::MUL
            || self.lexer.current_token().token == TokenType::DIV
        {
            let op = self.lexer.current_token().token;
            self.lexer.next_token();
            let right = self.factor();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::MUL => {
                            return Expr::Val(Val {
                                value: Literal::Number(n * m),
                            });
                        }
                        TokenType::DIV => {
                            return Expr::Val(Val {
                                value: Literal::Number(n / m),
                            });
                        }
                        _ => {}
                    },
                    (_, _) => {}
                },
                (_, _) => {}
            }
            left = Expr::BinOp(BinOp {
                left: Box::new(left),
                right: Box::new(right),
                operator: op,
            });
        }
        return left;
    }
    fn factor(&mut self) -> Expr {
        if self.lexer.current_token().token == TokenType::LITERAL {
            if let Some(val) = self.lexer.current_token().value {
                self.lexer.next_token();
                return Expr::Val(Val { value: val });
            }
        } else if self.lexer.current_token().token == TokenType::LPAREN {
            self.lexer.next_token();
            let expr = self.expr();
            if self.lexer.current_token().token != TokenType::RPAREN {
                panic!("Parser: Expected: ')'");
            }
            self.lexer.next_token();
            return expr;
        } else if self.lexer.current_token().token == TokenType::NEG {
            self.lexer.next_token();
            let argument = self.expr();
            match argument.clone() {
                Expr::Val(val) => match val.value {
                    Literal::Number(n) => {
                        return Expr::Val(Val {
                            value: Literal::Number(-n),
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
            return Expr::UnaryOp(UnaryOp {
                argument: Box::new(argument),
                operator: TokenType::NEG,
            });
        } else if self.lexer.current_token().token == TokenType::LOGNOT {
            self.lexer.next_token();
            let argument = self.expr();
            match argument.clone() {
                Expr::Val(val) => match val.value {
                    Literal::Bool(n) => {
                        return Expr::Val(Val {
                            value: Literal::Bool(!n),
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
            return Expr::UnaryOp(UnaryOp {
                argument: Box::new(argument),
                operator: TokenType::LOGNOT,
            });
        } else if self.lexer.current_token().token == TokenType::IDENT {
            let name = self.get_ident();
            self.lexer.next_token();
            match self.lexer.current_token().token {
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
                    self.lexer.next_token();
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
                    return Expr::VarMod(VarMod {
                        name,
                        value: Box::new(val),
                    });
                }
                _ => return Expr::Var(Var { name }),
            }
        }
        panic!(
            "Parser: Unexpected token: '{:?}'",
            self.lexer.current_token().token
        )
    }

    fn get_ident(&mut self) -> String {
        match self.lexer.current_token().value.unwrap() {
            Literal::Str(s) => s,
            _ => {
                panic!(
                    "Invalid name: {:?}",
                    self.lexer.current_token().value.unwrap()
                )
            }
        }
    }
}
