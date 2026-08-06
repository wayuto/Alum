mod bytecode;
mod compiler;
mod gvm;

pub use bytecode::*;
pub use compiler::*;
pub use gvm::GVM;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{lexer::Lexer, parser::Parser};

    fn run(src: &str) -> Option<Value> {
        let lexer = Lexer::new(src);
        let mut parser = Parser::new(lexer);
        let program = parser.parse().expect("parse failed");
        let bytecode = Compiler::new().compile(program);
        let mut vm = GVM::new(bytecode);
        vm.run();
        vm.result()
    }

    #[test]
    fn arithmetic() {
        let src = "1 + 2 * 3 - 4";
        assert_eq!(run(src), Some(Value::Int(3)));
    }

    #[test]
    fn float_arithmetic() {
        let src = "1.5 + 2.5 * 2.0";
        assert_eq!(run(src), Some(Value::Float(6.5)));
    }

    #[test]
    fn recursion_factorial() {
        let src = "fun fact(n: int): int {
    if n < 2 { return 1 }
    return n * fact(n - 1)
}
fact(10)
";
        assert_eq!(run(src), Some(Value::Int(3628800)));
    }

    #[test]
    fn loop_sum() {
        let src = "
var sum: int = 0
var i: int = 0
while i <= 100 {
    sum = sum + i
    i = i + 1
}
sum
";
        assert_eq!(run(src), Some(Value::Int(5050)));
    }

    #[test]
    fn func_with_locals() {
        let src = "fun pow(base: int, exp: int): int {
    var result: int = 1
    var i: int = 0
    while i < exp {
        result = result * base
        i = i + 1
    }
    return result
}
pow(2, 10)
";
        assert_eq!(run(src), Some(Value::Int(1024)));
    }
}
