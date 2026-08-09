#[path = "../src/compiler/mod.rs"]
mod compiler;

use compiler::lexer::Lexer;
use compiler::parser::Parser;
use compiler::visitor::TypeChecker;

fn dump(src: &str) -> String {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    match parser.parse() {
        Ok(prog) => format!("{}", prog),
        Err(e) => format!("PARSE ERROR: {}", e),
    }
}

fn assert_ok(src: &str) -> String {
    let out = dump(src);
    assert!(
        !out.starts_with("PARSE ERROR"),
        "parse failed for {src:?}: {out}"
    );
    out
}

fn assert_no_panic(src: &str) -> String {
    std::panic::catch_unwind(|| dump(src)).unwrap_or_else(|_| {
        panic!("parser panicked for {src:?}");
    })
}

fn check_ok(src: &str) {
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let mut ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"));
    let checker = TypeChecker::new();
    checker
        .check(&mut ast)
        .unwrap_or_else(|e| panic!("type check failed for {src:?}: {e}"));
}

#[test]
fn postfix_inc_dec_var() {
    assert!(!assert_ok("a++").contains("Inc(\"a\")") == false);
    assert!(assert_ok("a++").contains("Inc(\"a\")"));
    assert!(assert_ok("a--").contains("Dec(\"a\")"));
    assert!(assert_ok("++a").contains("Inc(\"a\")"));
    assert!(assert_ok("--a").contains("Dec(\"a\")"));
}

#[test]
fn compound_assign_var() {
    assert!(assert_ok("a += 1").contains("AddAssign(\"a\""));
    assert!(assert_ok("a -= 1").contains("SubAssign(\"a\""));
}

#[test]
fn index_inc_dec_and_compound() {
    assert!(assert_ok("a[0] += 1").starts_with("IndexAssign("));
    assert!(assert_ok("a[0] -= 1").starts_with("IndexAssign("));
    assert!(assert_ok("a[0]++").starts_with("IndexAssign("));
    assert!(assert_ok("a[0]--").starts_with("IndexAssign("));
    assert!(assert_ok("++a[0]").starts_with("IndexAssign("));
    assert!(assert_ok("--a[0]").starts_with("IndexAssign("));
}

#[test]
fn member_inc_dec_and_compound() {
    assert!(assert_ok("s.x += 1").starts_with("MemberAssign("));
    assert!(assert_ok("s.x -= 1").starts_with("MemberAssign("));
    assert!(assert_ok("s.x++").starts_with("MemberAssign("));
    assert!(assert_ok("s.x--").starts_with("MemberAssign("));
    assert!(assert_ok("++s.x").starts_with("MemberAssign("));
    assert!(assert_ok("--s.x").starts_with("MemberAssign("));
}

#[test]
fn deref_inc_dec_and_compound() {
    assert!(assert_ok("*p += 1").starts_with("DerefAssign("));
    assert!(assert_ok("*p -= 1").starts_with("DerefAssign("));
    assert!(assert_ok("(*p)++").starts_with("DerefAssign("));
    assert!(assert_ok("(*p)--").starts_with("DerefAssign("));
    assert!(assert_ok("++(*p)").starts_with("DerefAssign("));
    assert!(assert_ok("--(*p)").starts_with("DerefAssign("));
    assert!(assert_ok("*p++").starts_with("Deref("));
}

#[test]
fn nested_contexts() {
    for src in [
        "x = a[0]++",
        "x = a[0] += 1",
        "x = s.x++",
        "x = *p++",
        "foo(a[0]++, 1)",
        "foo(++a[0], 1)",
        "x = (*p)++",
        "x = ++s.x",
    ] {
        assert_no_panic(src);
        assert_ok(src);
    }
}

#[test]
fn no_cross_line_merge() {
    let out = dump("var x: int = 0\n++i");
    assert!(out.contains("GlobalVar(\"x\"") || out.contains("VarDecl(\"x\""));
    assert!(out.contains("Inc(\"i\")"), "got: {out}");
}

#[test]
fn no_cross_line_call_merge() {
    let out = dump("fun main(): int {\n    var p: *int = &arr0\n    (*p)++\n    return 0\n}");
    assert!(
        out.contains("Int(0)"),
        "RHS must not absorb next statement, got: {out}"
    );
}

#[test]
fn eof_safe() {
    for src in [
        "a++", "a--", "++a", "--a", "a += 1", "a[0]++", "(*p)++", "s.x--", "++s.x",
    ] {
        assert_no_panic(src);
    }
}

#[test]
fn type_check_end_to_end() {
    check_ok(
        "fun main(): int {\n\
         \x20   var arr: int[4] = [1,2,3,4]\n\
         \x20   arr[0] += 1\n\
         \x20   arr[1]--\n\
         \x20   ++arr[2]\n\
         \x20   var p: *int = &arr[0]\n\
         \x20   (*p)++\n\
         \x20   *p += 2\n\
         \x20   return 0\n\
         }",
    );
}
