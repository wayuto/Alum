use std::collections::HashMap;

use crate::{
    ast::{
        ArrayAccess, ArrayAssign, BinOp, Expr, Extern, For, FuncCall, FuncDecl, Goto, If, Label,
        Program, Return, Stmt, UnaryOp, Val, Var, VarDecl, VarMod, While,
    },
    lexer::{Lexer, LexerError},
    token::{Literal, Token, TokenType, VarType},
};

#[derive(Debug, Clone)]
pub enum ParserError {
    LexerError(LexerError),
    SyntaxError { message: String, row: usize, col: usize },
    UnexpectedChar { expected: Option<String>, found: char, row: usize, col: usize },
    UnknownType { row: usize, col: usize },
    TypeError { message: String, row: usize, col: usize },
}

impl From<LexerError> for ParserError {
    fn from(err: LexerError) -> Self {
        ParserError::LexerError(err)
    }
}

impl std::error::Error for ParserError {}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::LexerError(e) => write!(f, "{}", e),
            ParserError::SyntaxError { message, row, col } => {
                write!(f, "Syntax error at {}:{}: {}", row, col, message)
            }
            ParserError::UnexpectedChar { expected, found, row, col } => {
                if let Some(exp) = expected {
                    write!(f, "Unexpected char at {}:{}: expected '{}', found '{}'", row, col, exp, found)
                } else {
                    write!(f, "Unexpected char at {}:{}: '{}'", row, col, found)
                }
            }
            ParserError::UnknownType { row, col } => {
                write!(f, "Unknown type at {}:{}", row, col)
            }
            ParserError::TypeError { message, row, col } => {
                write!(f, "Type error at {}:{}: {}", row, col, message)
            }
        }
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    functions: HashMap<String, VarType>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
            functions: HashMap::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        self.lexer.next_token()?;
        let mut exprs: Vec<Expr> = Vec::new();
        while self.lexer.curr_tok().token != TokenType::EOF {
            exprs.push(self.ctrl()?);
        }
        Ok(Program { body: exprs })
    }
    fn ctrl(&mut self) -> Result<Expr, ParserError> {
        match self.lexer.curr_tok().token {
            TokenType::IF => {
                self.lexer.next_token()?;
                let cond = self.expr()?;
                let body = self.stmt()?;
                if self.lexer.curr_tok().token == TokenType::ELSE {
                    self.lexer.next_token()?;
                    let else_body = self.stmt()?;
                    match cond.clone() {
                        Expr::Val(val) => match val.value {
                            Literal::Bool(b) => {
                                if b {
                                    return Ok(body);
                                } else {
                                    return Ok(else_body);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    return Ok(Expr::If(If {
                        condition: Box::new(cond),
                        then_branch: Box::new(body),
                        else_branch: Some(Box::new(else_body)),
                    }));
                }
                match cond.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(b) => {
                            if b {
                                return Ok(body);
                            } else {
                                return Ok(Expr::Stmt(Stmt { body: vec![] }));
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(Expr::If(If {
                    condition: Box::new(cond),
                    then_branch: Box::new(body),
                    else_branch: None,
                }))
            }
            TokenType::WHILE => {
                self.lexer.next_token()?;
                let cond = self.expr()?;
                let body = self.stmt()?;
                match cond.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(b) => {
                            if b {
                                let lbl = format!("loop{:p}", &body);
                                return Ok(Expr::Stmt(Stmt {
                                    body: vec![
                                        Expr::Label(Label { name: lbl.clone() }),
                                        body,
                                        Expr::Goto(Goto { label: lbl }),
                                    ],
                                }));
                            } else {
                                return Ok(Expr::Stmt(Stmt { body: vec![] }));
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(Expr::While(While {
                    condition: Box::new(cond),
                    body: Box::new(body),
                }))
            }
            TokenType::FOR => {
                self.lexer.next_token()?;
                let init = self.get_ident()?;
                self.lexer.next_token()?;
                if self.lexer.curr_tok().token != TokenType::IN {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some("in".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                let iter = self.expr()?;
                let body = self.stmt()?;
                Ok(Expr::For(For {
                    init,
                    iter: Box::new(iter),
                    body: Box::new(body),
                }))
            }
            TokenType::PUB => {
                self.lexer.next_token()?;
                self.func_decl(true)
            }
            TokenType::FUNCDECL => self.func_decl(false),
            _ => self.stmt(),
        }
    }
    fn stmt(&mut self) -> Result<Expr, ParserError> {
        if self.lexer.curr_tok().token == TokenType::LBRACE {
            let mut exprs: Vec<Expr> = Vec::new();
            self.lexer.next_token()?;

            while self.lexer.curr_tok().token != TokenType::RBRACE {
                if self.lexer.curr_tok().token == TokenType::EOF {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some("}".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                exprs.push(self.ctrl()?);
            }

            self.lexer.next_token()?;
            return Ok(Expr::Stmt(Stmt { body: exprs }));
        }
        if self.lexer.curr_tok().token == TokenType::IF
            || self.lexer.curr_tok().token == TokenType::WHILE
            || self.lexer.curr_tok().token == TokenType::FUNCDECL
        {
            return self.ctrl();
        }
        self.expr()
    }
    fn expr(&mut self) -> Result<Expr, ParserError> {
        match self.lexer.curr_tok().token {
            TokenType::GOTO => {
                self.lexer.next_token()?;
                let name = self.get_ident()?;
                self.lexer.next_token()?;
                Ok(Expr::Goto(Goto { label: name }))
            }
            TokenType::VARDECL => {
                self.lexer.next_token()?;
                let name = self.get_ident()?;
                self.lexer.next_token()?;
                if self.lexer.curr_tok().token != TokenType::COLON {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some(":".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                let typ = match &self.lexer.curr_tok().token {
                    TokenType::Type(VarType::Int) => VarType::Int,
                    TokenType::Type(VarType::Float) => VarType::Float,
                    TokenType::Type(VarType::Bool) => VarType::Bool,
                    TokenType::Type(VarType::Str) => VarType::Str,
                    TokenType::Type(VarType::Array(n)) => VarType::Array(*n),
                    _ => {
                        return Err(ParserError::UnknownType {
                            row: self.lexer.curr_tok().row,
                            col: self.lexer.curr_tok().col,
                        });
                    }
                };
                self.lexer.next_token()?;
                if self.lexer.curr_tok().token != TokenType::EQ {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some("=".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                let value = self.expr()?;
                Ok(Expr::VarDecl(VarDecl {
                    name,
                    value: Box::new(value),
                    typ,
                }))
            }
            TokenType::RETURN => {
                self.lexer.next_token()?;
                let value = self.expr()?;
                Ok(Expr::Return(Return {
                    value: Some(Box::new(value)),
                }))
            }
            TokenType::EXTERN => {
                self.lexer.next_token()?;
                let func = self.get_ident()?;
                self.lexer.next_token()?;
                if self.lexer.curr_tok().token != TokenType::LPAREN {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some("(".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                let mut params: Vec<VarType> = Vec::new();
                while self.lexer.curr_tok().token != TokenType::RPAREN {
                    match self.lexer.curr_tok().token {
                        TokenType::Type(typ) => params.push(typ),
                        _ => {
                            return Err(ParserError::UnexpectedChar {
                                expected: Some("TYPE".to_string()),
                                found: self.lexer.curr_ch(),
                                row: self.lexer.curr_tok().row,
                                col: self.lexer.curr_tok().col,
                            });
                        }
                    }
                    self.lexer.next_token()?;
                }
                self.lexer.next_token()?;
                if self.lexer.curr_tok().token != TokenType::COLON {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some(":".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                let ret_type: VarType;
                match self.lexer.curr_tok().token {
                    TokenType::Type(typ) => ret_type = typ,
                    _ => {
                        return Err(ParserError::UnexpectedChar {
                            expected: Some("TYPE".to_string()),
                            found: self.lexer.curr_ch(),
                            row: self.lexer.curr_tok().row,
                            col: self.lexer.curr_tok().col,
                        });
                    }
                }
                self.lexer.next_token()?;
                self.functions.insert(func.clone(), ret_type.clone());
                Ok(Expr::Extern(Extern {
                    name: func,
                    params,
                    ret_type,
                }))
            }
            TokenType::IF | TokenType::WHILE | TokenType::LBRACE => self.ctrl(),
            _ => self.logical(),
        }
    }
    fn logical(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.comparison()?;
        while self.lexer.curr_tok().token == TokenType::LOGAND
            || self.lexer.curr_tok().token == TokenType::LOGOR
            || self.lexer.curr_tok().token == TokenType::LOGXOR
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token()?;
            let right = self.comparison()?;
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Int(n), Literal::Int(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n & m),
                                typ: VarType::Int,
                            }));
                        }
                        TokenType::LOGOR => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n | m),
                                typ: VarType::Int,
                            }));
                        }
                        TokenType::LOGXOR => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n ^ m),
                                typ: VarType::Int,
                            }));
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::LOGAND => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n & m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::LOGOR => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n | m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::LOGXOR => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n ^ m),
                                typ: VarType::Bool,
                            }));
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
        Ok(left)
    }
    fn comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.additive()?;
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
            self.lexer.next_token()?;
            let right = self.additive()?;
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Int(n), Literal::Int(m)) => match op.clone() {
                        TokenType::COMPEQ => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n == m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPNE => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n != m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPGT => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n > m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPGE => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n >= m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPLT => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n < m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPLE => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n <= m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::RANGE => {
                            let mut arr: Vec<Expr> = Vec::new();
                            for i in n..m {
                                arr.push(Expr::Val(Val {
                                    value: Literal::Int(i),
                                    typ: VarType::Int,
                                }));
                            }
                            return Ok(Expr::Val(Val {
                                value: Literal::Array((m - n) as usize, arr),
                                typ: VarType::Array(Some((m - n) as usize)),
                            }));
                        }
                        _ => {}
                    },
                    (Literal::Bool(n), Literal::Bool(m)) => match op.clone() {
                        TokenType::COMPAND => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n && m),
                                typ: VarType::Bool,
                            }));
                        }
                        TokenType::COMPOR => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(n || m),
                                typ: VarType::Bool,
                            }));
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
        Ok(left)
    }
    fn additive(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.term()?;
        while self.lexer.curr_tok().token == TokenType::ADD
            || self.lexer.curr_tok().token == TokenType::SUB
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token()?;
            let right = self.term()?;
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Int(n), Literal::Int(m)) => match op.clone() {
                        TokenType::ADD => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n + m),
                                typ: VarType::Int,
                            }));
                        }
                        TokenType::SUB => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n - m),
                                typ: VarType::Int,
                            }));
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
        Ok(left)
    }
    fn term(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.factor()?;
        while self.lexer.curr_tok().token == TokenType::MUL
            || self.lexer.curr_tok().token == TokenType::DIV
        {
            let op = self.lexer.curr_tok().token;
            self.lexer.next_token()?;
            let right = self.factor()?;
            match (left.clone(), right.clone()) {
                (Expr::Val(l), Expr::Val(r)) => match (l.value, r.value) {
                    (Literal::Int(n), Literal::Int(m)) => match op.clone() {
                        TokenType::MUL => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n * m),
                                typ: VarType::Int,
                            }));
                        }
                        TokenType::DIV => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(n / m),
                                typ: VarType::Int,
                            }));
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
        Ok(left)
    }
    fn factor(&mut self) -> Result<Expr, ParserError> {
        match self.lexer.curr_tok().token {
            TokenType::LITERAL(typ) => {
                if let Some(val) = self.lexer.curr_tok().value.clone() {
                    self.lexer.next_token()?;
                    Ok(Expr::Val(Val {
                        value: val,
                        typ: typ,
                    }))
                } else {
                    Err(ParserError::SyntaxError {
                        message: "expected literal value".to_string(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    })
                }
            }
            TokenType::LPAREN => {
                self.lexer.next_token()?;
                let expr = self.expr()?;
                if self.lexer.curr_tok().token != TokenType::RPAREN {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some(")".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
                self.lexer.next_token()?;
                Ok(expr)
            }
            TokenType::LBRACKET => {
                self.lexer.next_token()?;
                let mut array: Vec<Expr> = Vec::new();
                while self.lexer.curr_tok().token != TokenType::RBRACKET {
                    array.push(self.expr()?);
                    if self.lexer.curr_tok().token == TokenType::EOF {
                        return Err(ParserError::UnexpectedChar {
                            expected: Some("]".to_string()),
                            found: self.lexer.curr_ch(),
                            row: self.lexer.curr_tok().row,
                            col: self.lexer.curr_tok().col,
                        });
                    }
                }

                self.lexer.next_token()?;
                Ok(Expr::Val(Val {
                    value: Literal::Array(array.len(), array.clone()),
                    typ: VarType::Array(Some(array.len())),
                }))
            }
            TokenType::NEG => {
                self.lexer.next_token()?;
                let argument = self.expr()?;
                match argument.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Int(n) => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Int(-n),
                                typ: VarType::Int,
                            }));
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(Expr::UnaryOp(UnaryOp {
                    argument: Box::new(argument),
                    operator: TokenType::NEG,
                }))
            }
            TokenType::LOGNOT => {
                self.lexer.next_token()?;
                let argument = self.expr()?;
                match argument.clone() {
                    Expr::Val(val) => match val.value {
                        Literal::Bool(n) => {
                            return Ok(Expr::Val(Val {
                                value: Literal::Bool(!n),
                                typ: VarType::Bool,
                            }));
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(Expr::UnaryOp(UnaryOp {
                    argument: Box::new(argument),
                    operator: TokenType::LOGNOT,
                }))
            }
            TokenType::SIZEOF => {
                self.lexer.next_token()?;
                let argument = self.expr()?;
                Ok(Expr::UnaryOp(UnaryOp {
                    argument: Box::new(argument),
                    operator: TokenType::SIZEOF,
                }))
            }
            TokenType::IDENT => {
                let name = self.get_ident()?;
                self.lexer.next_token()?;
                match self.lexer.curr_tok().token {
                    TokenType::COLON => {
                        self.lexer.next_token()?;
                        Ok(Expr::Label(Label { name: name }))
                    }
                    TokenType::INC => {
                        self.lexer.next_token()?;
                        Ok(Expr::UnaryOp(UnaryOp {
                            argument: Box::new(Expr::Var(Var { name: name })),
                            operator: TokenType::INC,
                        }))
                    }
                    TokenType::DEC => {
                        self.lexer.next_token()?;
                        Ok(Expr::UnaryOp(UnaryOp {
                            argument: Box::new(Expr::Var(Var { name: name })),
                            operator: TokenType::DEC,
                        }))
                    }
                    TokenType::LPAREN => {
                        self.lexer.next_token()?;
                        let mut args: Vec<Expr> = Vec::new();
                        let ret_type = self.find_func_ret_type(&name)?;
                        while self.lexer.curr_tok().token != TokenType::RPAREN {
                            args.push(self.expr()?);
                        }
                        self.lexer.next_token()?;
                        Ok(Expr::FuncCall(FuncCall {
                            name,
                            args,
                            ret_type,
                        }))
                    }
                    TokenType::EQ => {
                        self.lexer.next_token()?;
                        let val = self.expr()?;
                        Ok(Expr::VarMod(VarMod {
                            name,
                            value: Box::new(val),
                        }))
                    }
                    TokenType::LBRACKET => {
                        self.lexer.next_token()?;
                        let offset = self.expr()?;
                        if self.lexer.curr_tok().token != TokenType::RBRACKET {
                            return Err(ParserError::UnexpectedChar {
                                expected: Some("]".to_string()),
                                found: self.lexer.curr_ch(),
                                row: self.lexer.curr_tok().row,
                                col: self.lexer.curr_tok().col,
                            });
                        }
                        self.lexer.next_token()?;
                        if self.lexer.curr_tok().token == TokenType::EQ {
                            self.lexer.next_token()?;
                            let value = self.expr()?;
                            Ok(Expr::ArrayAssign(ArrayAssign {
                                array: name,
                                offset: Box::new(offset),
                                value: Box::new(value),
                            }))
                        } else {
                            Ok(Expr::ArrayAccess(ArrayAccess {
                                array: name,
                                offset: Box::new(offset),
                            }))
                        }
                    }
                    _ => Ok(Expr::Var(Var { name })),
                }
            }
            _ => {
                Err(ParserError::SyntaxError {
                    message: format!("unexpected token: {:?}", self.lexer.curr_tok().token),
                    row: self.lexer.curr_tok().row,
                    col: self.lexer.curr_tok().col,
                })
            }
        }
    }

    fn get_ident(&mut self) -> Result<String, ParserError> {
        match self.lexer.curr_tok().value.as_ref() {
            Some(Literal::Str(s)) => Ok(s.clone()),
            Some(lit) => Err(ParserError::SyntaxError {
                message: format!("invalid name: {:?}", lit),
                row: self.lexer.curr_tok().row,
                col: self.lexer.curr_tok().col,
            }),
            None => Err(ParserError::SyntaxError {
                message: "expected identifier".to_string(),
                row: self.lexer.curr_tok().row,
                col: self.lexer.curr_tok().col,
            }),
        }
    }

    fn func_decl(&mut self, is_pub: bool) -> Result<Expr, ParserError> {
        self.lexer.next_token()?;
        let name = self.get_ident()?;
        let mut params: Vec<(String, VarType)> = Vec::new();
        self.lexer.next_token()?;
        if self.lexer.curr_tok().token != TokenType::LPAREN {
            return Err(ParserError::UnexpectedChar {
                expected: Some("(".to_string()),
                found: self.lexer.curr_ch(),
                row: self.lexer.curr_tok().row,
                col: self.lexer.curr_tok().col,
            });
        }
        self.lexer.next_token()?;
        while self.lexer.curr_tok().token != TokenType::RPAREN {
            if self.lexer.curr_tok().token == TokenType::EOF
                || self.lexer.curr_tok().token == TokenType::LBRACE
            {
                return Err(ParserError::UnexpectedChar {
                    expected: Some(")".to_string()),
                    found: self.lexer.curr_ch(),
                    row: self.lexer.curr_tok().row,
                    col: self.lexer.curr_tok().col,
                });
            }
            let name: String;
            let typ: VarType;
            if self.lexer.curr_tok().token == TokenType::IDENT {
                name = self.get_ident()?;
            } else if self.lexer.curr_tok().token == TokenType::RPAREN {
                break;
            } else {
                return Err(ParserError::UnexpectedChar {
                    expected: Some("IDENT".to_string()),
                    found: self.lexer.curr_ch(),
                    row: self.lexer.curr_tok().row,
                    col: self.lexer.curr_tok().col,
                });
            }
            self.lexer.next_token()?;
            if self.lexer.curr_tok().token == TokenType::COLON {
                self.lexer.next_token()?;
                match self.lexer.curr_tok().token {
                    TokenType::Type(vt) => {
                        typ = vt;
                    }
                    _ => {
                        return Err(ParserError::UnexpectedChar {
                            expected: Some("TYPE".to_string()),
                            found: self.lexer.curr_ch(),
                            row: self.lexer.curr_tok().row,
                            col: self.lexer.curr_tok().col,
                        });
                    }
                }
            } else {
                return Err(ParserError::UnexpectedChar {
                    expected: Some(":".to_string()),
                    found: self.lexer.curr_ch(),
                    row: self.lexer.curr_tok().row,
                    col: self.lexer.curr_tok().col,
                });
            }
            params.push((name, typ));
            self.lexer.next_token()?;
        }
        self.lexer.next_token()?;
        let ret_type: VarType;
        if self.lexer.curr_tok().token == TokenType::COLON {
            self.lexer.next_token()?;
            match self.lexer.curr_tok().token {
                TokenType::Type(vt) => {
                    ret_type = vt;
                }
                _ => {
                    return Err(ParserError::UnexpectedChar {
                        expected: Some("TYPE".to_string()),
                        found: self.lexer.curr_ch(),
                        row: self.lexer.curr_tok().row,
                        col: self.lexer.curr_tok().col,
                    });
                }
            }
        } else {
            return Err(ParserError::UnexpectedChar {
                expected: Some(":".to_string()),
                found: self.lexer.curr_ch(),
                row: self.lexer.curr_tok().row,
                col: self.lexer.curr_tok().col,
            });
        }
        self.functions.insert(name.clone(), ret_type.clone());
        self.lexer.next_token()?;
        let body = self.expr()?;
        Ok(Expr::FuncDecl(FuncDecl {
            name,
            params,
            body: Box::new(body),
            ret_type,
            is_pub,
        }))
    }

    fn find_func_ret_type(&self, name: &String) -> Result<VarType, ParserError> {
        self.functions.get(name).ok_or_else(|| ParserError::SyntaxError {
            message: format!("undefined function: '{}'", name),
            row: 0,
            col: 0,
        }).map(|t| t.to_owned())
    }
}
