use super::error::LexerError;
use crate::compiler::{
    Span,
    lexer::{FstringSeg, Lexer, Token},
};

impl<'a> Lexer<'a> {
    fn bump(&mut self) {
        self.current = self.chars.next();
        self.col += 1;
    }

    fn sw(&mut self) {
        while let Some(c) = self.current {
            if c.is_whitespace() {
                if c == '\n' {
                    self.line += 1;
                    self.col = 1;
                }
                self.bump()
            } else {
                break;
            }
        }
    }

    fn lex_int(&mut self) -> Result<isize, LexerError> {
        let mut buf = String::new();
        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }
        buf.parse::<isize>().map_err(|_| LexerError::InvalidNumber {
            line: self.line,
            col: self.col,
        })
    }

    fn lex_float(&mut self) -> Result<f64, LexerError> {
        let mut buf = String::new();

        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }

        if let Some('.') = self.current {
            buf.push('.');
            self.bump();
        } else {
            return Err(LexerError::InvalidNumber {
                line: self.line,
                col: self.col,
            });
        }

        while let Some(c) = self.current
            && c.is_ascii_digit()
        {
            buf.push(c);
            self.bump();
        }

        if let Some('e' | 'E') = self.current {
            buf.push(self.current.unwrap());
            self.bump();
            if let Some('+' | '-') = self.current {
                buf.push(self.current.unwrap());
                self.bump();
            }
            while let Some(c) = self.current
                && c.is_ascii_digit()
            {
                buf.push(c);
                self.bump();
            }
        }
        buf.parse::<f64>().map_err(|_| LexerError::InvalidNumber {
            line: self.line,
            col: self.col,
        })
    }

    fn lex_ident(&mut self) -> Result<String, LexerError> {
        let mut ident = String::new();

        if let Some(c) = self.current
            && (c.is_alphabetic() || c == '_')
        {
            ident.push(c);
            self.bump();
        } else {
            return Err(LexerError::UnexpectedChar {
                expected: None,
                found: self.current.unwrap(),
                line: self.line,
                col: self.col,
            });
        }

        while let Some(c) = self.current
            && (c.is_alphanumeric() || c == '_')
        {
            ident.push(c);
            self.bump();
        }

        Ok(ident)
    }

    fn lex_string(&mut self, quote: char) -> Result<String, LexerError> {
        let mut s = String::new();
        while self.current != Some(quote) {
            if self.current == Some('\0') {
                return Err(LexerError::UnclosedQuote {
                    line: self.line,
                    col: self.col,
                });
            }
            if self.current == Some('\\') {
                self.bump();
                match self.current {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some(c) => s.push(c),
                    None => {}
                }
                self.bump();
                continue;
            }
            s.push(self.current.unwrap());
            self.bump();
        }
        self.bump();
        Ok(s)
    }

    fn next_tok(&mut self) -> Result<(Token, Span), LexerError> {
        self.sw();
        let line = self.line;
        let col = self.col;
        let c = match self.current {
            None => return Ok((Token::EOF, Span::new(line, col))),
            Some(ch) => ch,
        };

        let tok = match c {
            '+' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::PLUSEQ
                } else if self.current == Some('+') {
                    self.bump();
                    Token::PLUSPLUS
                } else {
                    Token::PLUS
                }
            }
            '-' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::MINUSEQ
                } else if self.current == Some('-') {
                    self.bump();
                    Token::MINUSMINUS
                } else {
                    Token::MINUS
                }
            }
            '*' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::STAREQ
                } else {
                    Token::STAR
                }
            }
            '/' => {
                self.bump();
                if self.current == Some('/') {
                    while self.current != Some('\n') && self.current.is_some() {
                        self.bump();
                    }
                    return self.next_tok();
                }
                if self.current == Some('=') {
                    self.bump();
                    Token::SLASHEQ
                } else {
                    Token::SLASH
                }
            }
            '%' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::PERCENTEQ
                } else {
                    Token::PERCENT
                }
            }
            '.' => {
                self.bump();
                if self.current == Some('.') {
                    self.bump();
                    Token::DOTDOT
                } else {
                    Token::DOT
                }
            }
            '(' => {
                self.bump();
                Token::LPAREN
            }
            ')' => {
                self.bump();
                Token::RPAREN
            }
            '{' => {
                self.bump();
                Token::LBRACE
            }
            '}' => {
                self.bump();
                Token::RBRACE
            }
            '[' => {
                self.bump();
                Token::LBRACKET
            }
            ']' => {
                self.bump();
                Token::RBRACKET
            }
            '=' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::CEQ
                } else {
                    Token::EQ
                }
            }
            '!' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::NE
                } else {
                    Token::NOT
                }
            }
            '>' => {
                self.bump();
                if self.current == Some('>') {
                    self.bump();
                    if self.current == Some('=') {
                        self.bump();
                        Token::SHREQ
                    } else {
                        Token::SHR
                    }
                } else if self.current == Some('=') {
                    self.bump();
                    Token::GE
                } else {
                    Token::GT
                }
            }
            '<' => {
                self.bump();
                if self.current == Some('<') {
                    self.bump();
                    if self.current == Some('=') {
                        self.bump();
                        Token::SHLEQ
                    } else {
                        Token::SHL
                    }
                } else if self.current == Some('=') {
                    self.bump();
                    Token::LE
                } else {
                    Token::LT
                }
            }
            '&' => {
                self.bump();
                if self.current == Some('&') {
                    self.bump();
                    Token::AND
                } else if self.current == Some('=') {
                    self.bump();
                    Token::ANDEQ
                } else {
                    Token::LAND
                }
            }
            '|' => {
                self.bump();
                if self.current == Some('|') {
                    self.bump();
                    Token::OR
                } else if self.current == Some('=') {
                    self.bump();
                    Token::OREQ
                } else {
                    Token::LOR
                }
            }
            '^' => {
                self.bump();
                if self.current == Some('=') {
                    self.bump();
                    Token::XOREQ
                } else {
                    Token::XOR
                }
            }
            '~' => {
                self.bump();
                Token::BNOT
            }
            ':' => {
                self.bump();
                if self.current == Some(':') {
                    self.bump();
                    Token::COLONCOLON
                } else {
                    Token::COLON
                }
            }
            ',' => {
                self.bump();
                Token::COMMA
            }
            ';' => {
                self.bump();
                Token::SEMICOLON
            }
            '@' => {
                self.bump();
                Token::AT
            }
            '\\' => {
                self.bump();
                Token::LAMBDA
            }
            '\'' => {
                self.bump();
                Token::STRING(self.lex_string('\'')?)
            }
            '"' => {
                self.bump();
                Token::STRING(self.lex_string('"')?)
            }
            _ if c.is_ascii_digit() => {
                let mut lookahead = self.chars.clone();
                let mut is_float = false;

                while let Some(ch) = lookahead.next() {
                    if ch == '.' {
                        if let Some(next_char) = lookahead.next() {
                            if next_char.is_ascii_digit() {
                                is_float = true;
                            }
                        }
                        break;
                    } else if !ch.is_ascii_digit() {
                        break;
                    }
                }
                if is_float {
                    self.lex_float().map(Token::FLOAT)?
                } else {
                    self.lex_int().map(Token::INT)?
                }
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                if c == 'f' {
                    let mut lookahead = self.chars.clone();
                    if matches!(lookahead.next(), Some('"') | Some('\'')) {
                        self.bump();
                        let quote = self.current.unwrap();
                        self.bump();
                        let segs = self.lex_fstring(quote)?;
                        Token::FSTRING(segs)
                    } else {
                        let ident = self.lex_ident()?;
                        self.keyword(ident)
                    }
                } else {
                    let ident = self.lex_ident()?;
                    self.keyword(ident)
                }
            }
            _ => {
                let err = LexerError::UnexpectedChar {
                    expected: None,
                    found: c,
                    line: self.line,
                    col: self.col,
                };
                self.bump();
                return Err(err);
            }
        };

        Ok((tok, Span::new(line, col)))
    }

    fn keyword(&self, ident: String) -> Token {
        match ident.as_str() {
            "var" => Token::VAR,
            "cst" => Token::CST,
            "fun" => Token::FUN,
            "for" => Token::FOR,
            "in" => Token::IN,
            "extern" => Token::EXTERN,
            "import" => Token::IMPORT,
            "using" => Token::USING,
            "as" => Token::AS,
            "int" => Token::TYPE(ident),
            "float" => Token::TYPE(ident),
            "bool" => Token::TYPE(ident),
            "string" => Token::TYPE(ident),
            "arr" => Token::IDENT(ident),
            "void" => Token::TYPE(ident),
            "true" => Token::BOOL(true),
            "false" => Token::BOOL(false),
            "nil" => Token::NIL,
            "return" => Token::RET,
            "if" => Token::IF,
            "else" => Token::ELSE,
            "while" => Token::WHILE,
            "break" => Token::BREAK,
            "continue" => Token::CONTINUE,
            "typedef" => Token::TYPEDEF,
            "match" => Token::MATCH,
            "struct" => Token::STRUCT,
            "union" => Token::UNION,
            "enum" => Token::ENUM,
            _ => Token::IDENT(ident),
        }
    }

    fn lex_fstring(&mut self, quote: char) -> Result<Vec<FstringSeg>, LexerError> {
        let mut segs: Vec<FstringSeg> = Vec::new();
        let mut lit = String::new();

        fn flush(segs: &mut Vec<FstringSeg>, lit: &mut String) {
            if !lit.is_empty() {
                segs.push(FstringSeg::Lit(std::mem::take(lit)));
            }
        }

        while self.current != Some(quote) {
            match self.current {
                None | Some('\0') => {
                    return Err(LexerError::UnclosedQuote {
                        line: self.line,
                        col: self.col,
                    });
                }
                Some('\\') => {
                    self.bump();
                    match self.current {
                        Some('n') => lit.push('\n'),
                        Some('t') => lit.push('\t'),
                        Some('r') => lit.push('\r'),
                        Some(c) => lit.push(c),
                        None => {}
                    }
                    self.bump();
                }
                Some('{') => {
                    let next = self.chars.clone().next();
                    if next == Some('{') {
                        lit.push('{');
                        self.bump();
                        self.bump();
                    } else {
                        self.bump();
                        flush(&mut segs, &mut lit);
                        let raw = self.read_expr_body()?;
                        segs.push(FstringSeg::Expr(raw));
                    }
                }
                Some('}') => {
                    let next = self.chars.clone().next();
                    if next == Some('}') {
                        lit.push('}');
                        self.bump();
                        self.bump();
                    } else {
                        return Err(LexerError::UnexpectedChar {
                            expected: None,
                            found: '}',
                            line: self.line,
                            col: self.col,
                        });
                    }
                }
                Some(c) => {
                    lit.push(c);
                    self.bump();
                }
            }
        }
        flush(&mut segs, &mut lit);
        self.bump();
        Ok(segs)
    }

    fn read_expr_body(&mut self) -> Result<String, LexerError> {
        let mut buf = String::new();
        let mut depth: isize = 0;
        let mut quote: Option<char> = None;

        loop {
            match self.current {
                None | Some('\0') => {
                    return Err(LexerError::UnclosedQuote {
                        line: self.line,
                        col: self.col,
                    });
                }
                Some('\\') => {
                    buf.push('\\');
                    self.bump();
                    if let Some(c) = self.current {
                        buf.push(c);
                        self.bump();
                    }
                }
                Some(c @ ('"' | '\'')) => match quote {
                    Some(q) if q == c => {
                        buf.push(c);
                        self.bump();
                        quote = None;
                    }
                    Some(_) => {
                        buf.push(c);
                        self.bump();
                    }
                    None => {
                        buf.push(c);
                        self.bump();
                        quote = Some(c);
                    }
                },
                Some('{') => {
                    buf.push('{');
                    self.bump();
                    depth += 1;
                }
                Some('}') => {
                    if depth == 0 {
                        self.bump();
                        break;
                    }
                    buf.push('}');
                    self.bump();
                    depth -= 1;
                }
                Some(c) => {
                    buf.push(c);
                    self.bump();
                }
            }
        }
        Ok(buf)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(Token, Span), LexerError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.sw();
        match self.current {
            None => None,
            _ => Some(self.next_tok()),
        }
    }
}
