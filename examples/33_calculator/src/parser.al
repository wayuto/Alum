using memory::malloc
using lexer::{Lexer, Token, TokenType, next_token}

cst(pub) NUM: int = 0
cst(pub) BINOP: int = 1

struct(pub) Node {
    kind: int,
    op: int,
    value: int,
    left: *Node,
    right: *Node
}

struct(pub) Parser {
    lex: *Lexer,
    cur: *Token
}

fun advance(p: *Parser) {
    var t: *Token = next_token(p.lex)
    p.cur = t;
}

fun(pub) new(lx: *lexer::Lexer): *Parser {
    var p: *Parser = malloc(16)@*Parser
    p.lex = lx
    p.cur = nil@*Token
    advance(p)
    return p
}

fun(pub) parse_factor(p: *Parser): *Node {
    if p.cur.tok == MINUS {
        advance(p)
        var zero: *Node = Node { kind: NUM, op: 0, value: 0, left: nil@*Node, right: nil@*Node }
        return Node { kind: BINOP, op: MINUS, value: 0, left: zero, right: parse_factor(p) }
    }
    if p.cur.tok == LPAREN {
        advance(p)
        var e = parse_expr(p)
        if p.cur.tok == RPAREN {
            advance(p)
        }
        return e
    }
    if p.cur.tok == INT {
        var node: *Node = Node { kind: NUM, op: 0, value: p.cur.val, left: nil@*Node, right: nil@*Node }
        advance(p)
        return node
    }
    return Node { kind: NUM, op: 0, value: 0, left: nil@*Node, right: nil@*Node }
}

fun(pub) parse_term(p: *Parser): *Node {
    var l = parse_factor(p)
    while p.cur.tok == STAR || p.cur.tok == SLASH {
        var op: int = p.cur.tok
        advance(p)
        l = Node { kind: BINOP, op: op, value: 0, left: l, right: parse_factor(p) }
    }
    return l
}

fun(pub) parse_expr(p: *Parser): *Node {
    var l = parse_term(p)
    while p.cur.tok == PLUS || p.cur.tok == MINUS {
        var op: int = p.cur.tok
        advance(p)
        l = Node { kind: BINOP, op: op, value: 0, left: l, right: parse_term(p) }
    }
    return l
}
