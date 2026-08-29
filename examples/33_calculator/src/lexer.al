using maybe::Maybe
using string::strlen
using memory::malloc

enum(pub) TokenType {
    INT,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    LPAREN,
    RPAREN,
    EOF
}

struct(pub) Token {
    tok: TokenType,
    val: int
}

struct(pub) Lexer {
    src: string,
    pos: int
}

fun(pub) new(src: string): Lexer {
    return Lexer {
        src: src,
        pos: 0
    }
}

fun(pub) bump(self: *Lexer): int {
    if self.pos < strlen(self.src) {
        var c: int = self.src[self.pos]
        self.pos = self.pos + 1
        return c
    }
    return 0 - 1
}

fun(pub) peek(self: *Lexer): Maybe<int> {
    if self.pos < strlen(self.src) {
        return Maybe { tag: Just, value: self.src[self.pos] }
    }
    return Maybe { tag: Nothing, value: nil@int }
}

fun mk_tok<T>(tok: TokenType, val: T): *Token {
    var t: *Token = malloc(16)@*Token
    t.tok = tok
    t.val = val
    return t
}

fun(pub) next_token(self: *Lexer): *Token {
    var curr = bump(self);
    while curr == ' ' || curr == '\n' {
        curr = bump(self);
    }
    if curr == 0 - 1 {
        return mk_tok(EOF, nil@int)
    }
    match curr {
        '0'..'9'+1: {
            var n: int = curr - '0'
            curr = bump(self);
            while curr >= '0' && curr <= '9' {
                n = n * 10 + (curr - '0')
                curr = bump(self);
            }
            if curr != -1 {
                self.pos = self.pos - 1
            }
            var t: *Token = mk_tok(INT, n)
            return t
        }
        '(': mk_tok(LPAREN, nil@int)
        ')': mk_tok(RPAREN, nil@int)
        '+': mk_tok(PLUS, nil@int)
        '-': mk_tok(MINUS, nil@int)
        '*': mk_tok(STAR, nil@int)
        '/': mk_tok(SLASH, nil@int)
        _: mk_tok(EOF, nil@int)
    }
}
