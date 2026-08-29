using lexer::TokenType
using parser::Node
using memory::free

fun(pub) eval(n: *Node): int {
    if n.kind == parser::NUM {
        return n.value
    }

    var l = eval(n.left)
    var r = eval(n.right)
    match n.op {
        PLUS: l + r
        MINUS: l - r
        STAR: l * r
        SLASH: l / r
        _: l
    }
}