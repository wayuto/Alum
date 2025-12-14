use crate::{
    ast::{
        ArrayAccess, ArrayAssign, BinOp, Expr, Extern, For, FuncCall, FuncDecl, Goto, If, Label,
        Program, Return, Stmt, UnaryOp, Val, Var, VarDecl, VarMod, While,
    },
    error::GosError,
    lexer::Lexer,
    token::{Literal, TokenType, VarType},
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
        while self.lexer.curr_tok().token != TokenType::EOF {
            exprs.push(self.ctrl());
        }
        Program { body: exprs }
    }
    fn ctrl(&mut self) -> Expr {
        match self.lexer.curr_tok().token {
            TokenType::IF => {
                self.lexer.next_token();
                let cond = self.expr();
                let body = self.stmt();
                if self.lexer.curr_tok().token == TokenType::ELSE {
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
            TokenType::FOR => {
                self.lexer.next_token();
                let init = self.get_ident();
                self.lexer.next_token();
                if self.lexer.curr_tok().token != TokenType::IN {
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some("in"), self.lexer.curr_ch());
                }
                self.lexer.next_token();
                let iter = self.expr();
                let body = self.stmt();
                Expr::For(For {
                    init,
                    iter: Box::new(iter),
                    body: Box::new(body),
                })
            }
            TokenType::PUB => {
                self.lexer.next_token();
                self.func_decl(true)
            }
            TokenType::FUNCDECL => self.func_decl(false),
            _ => self.stmt(),
        }
    }
    fn stmt(&mut self) -> Expr {
        if self.lexer.curr_tok().token == TokenType::LBRACE {
            let mut exprs: Vec<Expr> = Vec::new();
            self.lexer.next_token();

            while self.lexer.curr_tok().token != TokenType::RBRACE {
                if self.lexer.curr_tok().token == TokenType::EOF {
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some("{"), self.lexer.curr_ch());
                    err.panic();
                }
                exprs.push(self.ctrl());
            }

            self.lexer.next_token();
            return Expr::Stmt(Stmt { body: exprs });
        }
        if self.lexer.curr_tok().token == TokenType::IF
            || self.lexer.curr_tok().token == TokenType::WHILE
            || self.lexer.curr_tok().token == TokenType::FUNCDECL
        {
            return self.ctrl();
        }
        self.expr()
    }
    fn expr(&mut self) -> Expr {
        match self.lexer.curr_tok().token {
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
                if self.lexer.curr_tok().token != TokenType::COLON {
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some(":"), self.lexer.curr_ch());
                    err.panic();
                }
                self.lexer.next_token();
                let typ = match &self.lexer.curr_tok().token {
                    TokenType::Type(VarType::Number) => VarType::Number,
                    TokenType::Type(VarType::Bool) => VarType::Bool,
                    TokenType::Type(VarType::Str) => VarType::Str,
                    TokenType::Type(VarType::Array(n)) => VarType::Array(*n),
                    _ => {
                        let mut err =
                            GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                        err.unknown_type();
                        err.panic();
                        panic!()
                    }
                };
                self.lexer.next_token();
                if self.lexer.curr_tok().token != TokenType::EQ {
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some("="), self.lexer.curr_ch());
                    err.panic();
                }
                self.lexer.next_token();
                let value = self.expr();
                Expr::VarDecl(VarDecl {
                    name,
                    value: Box::new(value),
                    typ,
                })
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
            TokenType::IF | TokenType::WHILE | TokenType::LBRACE => self.ctrl(),
            _ => self.logical(),
        }
    }
    fn logical(&mut self) -> Expr {
        let mut left = self.comparison();
        while self.lexer.curr_tok().token == TokenType::LOGAND
            || self.lexer.curr_tok().token == TokenType::LOGOR
            || self.lexer.curr_tok().token == TokenType::LOGXOR
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token();
            let right = self.comparison();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Expr::Val(Val {
                                value: Literal::Number(n & m),
                                typ: VarType::Number,
                            });
                        }
                        TokenType::LOGOR => {
                            return Expr::Val(Val {
                                value: Literal::Number(n | m),
                                typ: VarType::Number,
                            });
                        }
                        TokenType::LOGXOR => {
                            return Expr::Val(Val {
                                value: Literal::Number(n ^ m),
                                typ: VarType::Number,
                            });
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n & m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::LOGOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n | m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::LOGXOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n ^ m),
                                typ: VarType::Bool,
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
        while self.lexer.curr_tok().token == TokenType::COMPEQ
            || self.lexer.curr_tok().token == TokenType::COMPNE
            || self.lexer.curr_tok().token == TokenType::COMPLT
            || self.lexer.curr_tok().token == TokenType::COMPLE
            || self.lexer.curr_tok().token == TokenType::COMPGT
            || self.lexer.curr_tok().token == TokenType::COMPGE
            || self.lexer.curr_tok().token == TokenType::COMPAND
            || self.lexer.curr_tok().token == TokenType::COMPOR
            || self.lexer.curr_tok().token == TokenType::RANGE
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token();
            let right = self.additive();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::COMPEQ => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n == m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPNE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n != m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPGT => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n > m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPGE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n >= m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPLT => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n < m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPLE => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n <= m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::RANGE => {
                            let mut arr: Vec<Expr> = Vec::new();
                            for i in n..m {
                                arr.push(Expr::Val(Val {
                                    value: Literal::Number(i),
                                    typ: VarType::Number,
                                }));
                            }
                            return Expr::Val(Val {
                                value: Literal::Array((m - n) as usize, arr),
                                typ: VarType::Array(Some((m - n) as usize)),
                            });
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::COMPAND => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n && m),
                                typ: VarType::Bool,
                            });
                        }
                        TokenType::COMPOR => {
                            return Expr::Val(Val {
                                value: Literal::Bool(n || m),
                                typ: VarType::Bool,
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
        while self.lexer.curr_tok().token == TokenType::ADD
            || self.lexer.curr_tok().token == TokenType::SUB
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token();
            let right = self.term();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::ADD => {
                            return Expr::Val(Val {
                                value: Literal::Number(n + m),
                                typ: VarType::Number,
                            });
                        }
                        TokenType::SUB => {
                            return Expr::Val(Val {
                                value: Literal::Number(n - m),
                                typ: VarType::Number,
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
        while self.lexer.curr_tok().token == TokenType::MUL
            || self.lexer.curr_tok().token == TokenType::DIV
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token();
            let right = self.factor();
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Number(n), Literal::Number(m)) => match op.clone() {
                        TokenType::MUL => {
                            return Expr::Val(Val {
                                value: Literal::Number(n * m),
                                typ: VarType::Number,
                            });
                        }
                        TokenType::DIV => {
                            return Expr::Val(Val {
                                value: Literal::Number(n / m),
                                typ: VarType::Number,
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
        match self.lexer.curr_tok().token {
            TokenType::LITERAL(typ) => {
                if let Some(val) = self.lexer.curr_tok().value {
                    self.lexer.next_token();
                    return Expr::Val(Val {
                        value: val,
                        typ: typ,
                    });
                } else {
                    panic!()
                }
            }
            TokenType::LPAREN => {
                self.lexer.next_token();
                let expr = self.expr();
                if self.lexer.curr_tok().token != TokenType::RPAREN {
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some(")"), self.lexer.curr_ch());
                    err.panic();
                }
                self.lexer.next_token();
                return expr;
            }
            TokenType::LBRACKET => {
                self.lexer.next_token();
                let mut array: Vec<Expr> = Vec::new();
                while self.lexer.curr_tok().token != TokenType::RBRACKET {
                    array.push(self.expr());
                    if self.lexer.curr_tok().token == TokenType::EOF {
                        let mut err =
                            GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                        err.unexpected_char(Some("]"), self.lexer.curr_ch());
                        err.panic();
                    }
                }

                self.lexer.next_token();
                return Expr::Val(Val {
                    value: Literal::Array(array.len(), array.clone()),
                    typ: VarType::Array(Some(array.len())),
                });
            }
            TokenType::NEG => {
                self.lexer.next_token();
                let argument = self.expr();
                match argument.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Number(n) => {
                            return Expr::Val(Val {
                                value: Literal::Number(-n),
                                typ: VarType::Number,
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
            }
            TokenType::LOGNOT => {
                self.lexer.next_token();
                let argument = self.expr();
                match argument.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(n) => {
                            return Expr::Val(Val {
                                value: Literal::Bool(!n),
                                typ: VarType::Bool,
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
            }
            TokenType::SIZEOF => {
                self.lexer.next_token();
                let argument = self.expr();
                return Expr::UnaryOp(UnaryOp {
                    argument: Box::new(argument),
                    operator: TokenType::SIZEOF,
                });
            }
            TokenType::IDENT => {
                let name = self.get_ident();
                self.lexer.next_token();
                match self.lexer.curr_tok().token {
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
                        while self.lexer.curr_tok().token != TokenType::RPAREN {
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
                    TokenType::LBRACKET => {
                        self.lexer.next_token();
                        let offset = self.expr();
                        if self.lexer.curr_tok().token != TokenType::RBRACKET {
                            let mut err =
                                GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                            err.unexpected_char(Some("]"), self.lexer.curr_ch());
                            err.panic();
                        }
                        self.lexer.next_token();
                        if self.lexer.curr_tok().token == TokenType::EQ {
                            self.lexer.next_token();
                            let value = self.expr();
                            return Expr::ArrayAssign(ArrayAssign {
                                array: name,
                                offset: Box::new(offset),
                                value: Box::new(value),
                            });
                        } else {
                            return Expr::ArrayAccess(ArrayAccess {
                                array: name,
                                offset: Box::new(offset),
                            });
                        }
                    }
                    _ => return Expr::Var(Var { name }),
                }
            }
            _ => {
                let err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                err.panic();
                panic!()
            }
        }
    }

    fn get_ident(&mut self) -> String {
        match self.lexer.curr_tok().value.unwrap() {
            Literal::Str(s) => s,
            _ => {
                let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                err.invalid_name(self.lexer.curr_tok().value.unwrap());
                err.panic();
                panic!()
            }
        }
    }

    fn func_decl(&mut self, is_pub: bool) -> Expr {
        self.lexer.next_token();
        let name = self.get_ident();
        let mut params: Vec<(String, VarType)> = Vec::new();
        self.lexer.next_token();
        if self.lexer.curr_tok().token != TokenType::LPAREN {
            let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
            err.unexpected_char(Some("("), self.lexer.curr_ch());
            err.panic();
        }
        self.lexer.next_token();
        while self.lexer.curr_tok().token != TokenType::RPAREN {
            if self.lexer.curr_tok().token == TokenType::EOF
                || self.lexer.curr_tok().token == TokenType::LBRACE
            {
                let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                err.unexpected_char(Some(")"), self.lexer.curr_ch());
                err.panic();
            }
            let name: String;
            let typ: VarType;
            if self.lexer.curr_tok().token == TokenType::IDENT {
                name = self.get_ident();
            } else if self.lexer.curr_tok().token == TokenType::RPAREN {
                break;
            } else {
                let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                err.unexpected_char(Some("INDET"), self.lexer.curr_ch());
                err.panic();
                panic!()
            }
            self.lexer.next_token();
            if self.lexer.curr_tok().token == TokenType::COLON {
                self.lexer.next_token();
                match self.lexer.curr_tok().token {
                    TokenType::Type(vt) => {
                        typ = vt;
                    }
                    _ => {
                        let mut err =
                            GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                        err.unexpected_char(Some("TYPE"), self.lexer.curr_ch());
                        err.panic();
                        panic!()
                    }
                }
            } else {
                let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                err.unexpected_char(Some(":"), self.lexer.curr_ch());
                err.panic();
                panic!()
            }
            params.push((name, typ));
            self.lexer.next_token();
        }
        self.lexer.next_token();
        let ret_type: VarType;
        if self.lexer.curr_tok().token == TokenType::COLON {
            self.lexer.next_token();
            match self.lexer.curr_tok().token {
                TokenType::Type(vt) => {
                    ret_type = vt;
                }
                _ => {
                    println!("{:?}", self.lexer.curr_tok().token);
                    let mut err =
                        GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
                    err.unexpected_char(Some("TYPE"), self.lexer.curr_ch());
                    err.panic();
                    panic!();
                }
            }
        } else {
            let mut err = GosError::new(self.lexer.curr_tok().row, self.lexer.curr_tok().col);
            err.unexpected_char(Some(":"), self.lexer.curr_ch());
            err.panic();
            panic!();
        }
        self.lexer.next_token();
        let body = self.stmt();
        Expr::FuncDecl(FuncDecl {
            name,
            params,
            body: Box::new(body),
            ret_type,
            is_pub,
        })
    }
}
