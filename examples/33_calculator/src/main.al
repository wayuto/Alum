using io::{println, input}
using lexer::Lexer
using parser::Parser
using walker::eval

fun main(): int {
    while true {
        cst expr = input("> ")
        if expr == "q" break
        var lexer = lexer::new(expr)
        var parser = parser::new(&lexer)
        var tree = parser::parse_expr(parser)
        println(f"{eval(tree)}")
    }
    return 0
}
