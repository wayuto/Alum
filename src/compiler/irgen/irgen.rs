use super::context::Context;
use super::ir::{IRConst, IRGlobalVar, IRProgram, IRType, Op};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use ordered_float::OrderedFloat;
use std::mem::take;

impl IRGen {
    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        let program = self.lambda2function(program);
        self.program_body = program.body.clone();

        super::purity::check_pure_functions(&self.program_body)?;

        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, _) => {
                    if type_params.is_empty() {
                        self.func_decl(
                            name.clone(),
                            attrs.clone(),
                            params.clone(),
                            ret_type.clone(),
                        )?;
                        self.func_high_returns
                            .insert(name.clone(), ret_type.clone());
                    } else {
                        self.generic_funcs.insert(
                            name.clone(),
                            (
                                type_params.clone(),
                                params.clone(),
                                ret_type.clone(),
                                body.clone(),
                            ),
                        );
                    }
                }
                Expr::ExternVar(name, ty, _) => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::Struct(name, type_params, fields, _) => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union(name, type_params, fields, _) => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum(name, members, _) => {
                    self.enums.insert(name.clone(), members.clone());
                }
                _ => {}
            }
        }

        let const_decls: Vec<Expr> = program
            .body
            .iter()
            .filter(|e| matches!(e, Expr::ConstDecl(..)))
            .cloned()
            .collect();
        self.store_global_consts(&const_decls)?;
        self.store_global_vars(&program.body)?;

        for expr in program.body {
            match expr {
                Expr::FuncDecl(name, _, type_params, params, _, body, _) => {
                    if type_params.is_empty() {
                        self.compile_fn(name, params, *body)?;
                    }
                }
                Expr::ConstDecl(_, _, _, _, _) | Expr::GlobalVar(_, _, _, _, _) => {}
                Expr::Int(_, _)
                | Expr::Float(_, _)
                | Expr::Bool(_, _)
                | Expr::String(_, _)
                | Expr::Nil(_)
                | Expr::Var(_, _) => {
                    let mut ctx = Context::new("_global".to_string());
                    ctx.enter_scope();
                    self.compile_expr(
                        Expr::VarDecl(
                            "_global".to_string(),
                            Type::Primitive(Primitive::Int),
                            Box::new(expr),
                            crate::compiler::Span::new(0, 0),
                        ),
                        &mut ctx,
                    )?;
                }
                _ => {}
            }
        }

        Ok(IRProgram {
            functions: take(&mut self.functions),
            constants: take(&mut self.constants),
            extern_vars: take(&mut self.extern_vars).into_keys().collect(),
            global_vars: take(&mut self.global_emits),
        })
    }

    pub(super) fn store_global_vars(&mut self, body: &[Expr]) -> Result<(), CodeGenError> {
        for expr in body {
            match expr {
                Expr::GlobalVar(name, is_pub, ty, init, _) => {
                    let value = match init {
                        Some(init) => match self.eval_const(init) {
                            Some((cv, _)) => Some(cv),
                            None => {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "initializer of global variable '{}' is not a compile-time constant",
                                        name
                                    ),
                                });
                            }
                        },
                        None => None,
                    };
                    let ir_type = if matches!(ty, Type::Unknown) {
                        value
                            .as_ref()
                            .map(|cv| match cv {
                                IRConst::Float(_) => IRType::Float,
                                _ => IRType::Int,
                            })
                            .unwrap_or(IRType::Int)
                    } else {
                        Context::type2ir_type(ty)
                    };
                    if !matches!(ir_type, IRType::Int | IRType::Float | IRType::Bool) {
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "unsupported type '{}' for global variable '{}' (only int/float/bool)",
                                ty, name
                            ),
                        });
                    }
                    self.global_storage
                        .insert(name.clone(), (ir_type.clone(), value.clone(), *is_pub));
                    self.global_emits.push(IRGlobalVar {
                        name: name.clone(),
                        value,
                        is_pub: *is_pub,
                    });
                }
                Expr::ConstDecl(name, _, _, is_pub, _) => {
                    if *is_pub {
                        if let Some((cv, ct)) = self.globals.get(name).cloned() {
                            if matches!(&cv, IRConst::Str(_) | IRConst::Array(_)) {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "unsupported value for global constant '{}' (only int/float/bool can be exported)",
                                        name
                                    ),
                                });
                            }
                            let _ = ct;
                            self.global_emits.push(IRGlobalVar {
                                name: name.clone(),
                                value: Some(cv),
                                is_pub: true,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn store_global_consts(&mut self, exprs: &[Expr]) -> Result<(), CodeGenError> {
        let mut pending: Vec<(String, Expr)> = exprs
            .iter()
            .filter_map(|e| match e {
                Expr::ConstDecl(name, _, value, _, _) => {
                    Some((name.clone(), value.as_ref().clone()))
                }
                _ => None,
            })
            .collect();
        let mut progressed = true;
        while !pending.is_empty() && progressed {
            progressed = false;
            let mut next = Vec::new();
            for (name, value) in pending {
                if let Some(cv) = self.eval_const(&value) {
                    self.globals.insert(name, cv);
                    progressed = true;
                } else {
                    next.push((name, value));
                }
            }
            pending = next;
        }
        for (name, _) in pending {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "initializer of constant '{}' is not a compile-time constant",
                    name
                ),
            });
        }
        Ok(())
    }

    fn eval_const(&self, expr: &Expr) -> Option<(IRConst, IRType)> {
        match expr {
            Expr::Int(n, _) => Some((IRConst::Int(*n as i64), IRType::Int)),
            Expr::Float(f, _) => Some((IRConst::Float(OrderedFloat(*f)), IRType::Float)),
            Expr::String(s, _) => Some((IRConst::Str(s.clone()), IRType::String)),
            Expr::Bool(b, _) => Some((IRConst::Int(if *b { 1 } else { 0 }), IRType::Bool)),
            Expr::Nil(_) => Some((IRConst::Int(0), IRType::Int)),
            Expr::Var(name, _) => self.globals.get(name).cloned(),
            Expr::Neg(e, _) => match self.eval_const(e)? {
                (IRConst::Int(v), IRType::Int) => Some((IRConst::Int(-v), IRType::Int)),
                _ => None,
            },
            Expr::FNeg(e, _) => match self.eval_const(e)? {
                (IRConst::Float(v), IRType::Float) => Some((IRConst::Float(-v), IRType::Float)),
                _ => None,
            },
            Expr::Add(l, r, _)
            | Expr::Sub(l, r, _)
            | Expr::Mul(l, r, _)
            | Expr::Div(l, r, _)
            | Expr::Mod(l, r, _)
            | Expr::FAdd(l, r, _)
            | Expr::FSub(l, r, _)
            | Expr::FMul(l, r, _)
            | Expr::FDiv(l, r, _) => {
                let (lc, lt) = self.eval_const(l)?;
                let (rc, rt) = self.eval_const(r)?;
                if matches!(lt, IRType::Float) || matches!(rt, IRType::Float) {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Float(a), IRConst::Float(b)) => (a.into_inner(), b.into_inner()),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::FAdd(..) | Expr::Add(..) => a + b,
                        Expr::FSub(..) | Expr::Sub(..) => a - b,
                        Expr::FMul(..) | Expr::Mul(..) => a * b,
                        Expr::FDiv(..) | Expr::Div(..) => a / b,
                        _ => return None,
                    };
                    Some((IRConst::Float(OrderedFloat(v)), IRType::Float))
                } else {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Int(a), IRConst::Int(b)) => (a, b),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::Add(..) => a.wrapping_add(b),
                        Expr::Sub(..) => a.wrapping_sub(b),
                        Expr::Mul(..) => a.wrapping_mul(b),
                        Expr::Div(..) => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_div(b)
                        }
                        Expr::Mod(..) => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_rem(b)
                        }
                        _ => return None,
                    };
                    Some((IRConst::Int(v), IRType::Int))
                }
            }
            _ => self.eval_const_vm(expr),
        }
    }

    /// Slow-path compile-time evaluation: compile the pure function bodies from
    /// the source program plus the target expression into bytecode, run it on the
    /// GosVM, and translate the resulting value. Returns `None` if the expression
    /// is not (purely) compile-time evaluable.
    fn eval_const_vm(&self, expr: &Expr) -> Option<(IRConst, IRType)> {
        use crate::compiler::bytecode::{Compiler, GVM};

        let mut body: Vec<Expr> = self
            .program_body
            .iter()
            .filter(|e| {
                matches!(e, Expr::FuncDecl(_, attrs, ..) if attrs.is_pure && !attrs.is_external)
            })
            .cloned()
            .collect();
        body.push(expr.clone());
        let program = Program { body };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let bc = Compiler::new().compile(program);
            let mut vm = GVM::new(bc);
            vm.run();
            vm.result()
        }))
        .ok()
        .flatten()?;

        match result {
            crate::compiler::bytecode::Value::Int(i) => Some((IRConst::Int(i), IRType::Int)),
            crate::compiler::bytecode::Value::Float(f) => {
                Some((IRConst::Float(OrderedFloat(f)), IRType::Float))
            }
            crate::compiler::bytecode::Value::Str(s) => Some((IRConst::Str(s), IRType::String)),
            crate::compiler::bytecode::Value::Bool(b) => {
                Some((IRConst::Int(if b { 1 } else { 0 }), IRType::Bool))
            }
            _ => None,
        }
    }

    pub(super) fn get_const_index(&mut self, constant: IRConst) -> usize {
        if let Some(&index) = self.constant_pool.get(&constant) {
            return index;
        }
        let index = self.constants.len();
        self.constants.push(constant.clone());
        self.constant_pool.insert(constant, index);
        index
    }

    pub(super) fn get_binop_parts(expr: Expr) -> Result<(Op, Box<Expr>, Box<Expr>), CodeGenError> {
        let _span = crate::compiler::Span::new(0, 0);
        match expr {
            Expr::Add(l, r, _) => Ok((Op::Add, l, r)),
            Expr::Sub(l, r, _) => Ok((Op::Sub, l, r)),
            Expr::Mul(l, r, _) => Ok((Op::Mul, l, r)),
            Expr::Div(l, r, _) => Ok((Op::Div, l, r)),
            Expr::Mod(l, r, _) => Ok((Op::Mod, l, r)),
            Expr::FAdd(l, r, _) => Ok((Op::FAdd, l, r)),
            Expr::FSub(l, r, _) => Ok((Op::FSub, l, r)),
            Expr::FMul(l, r, _) => Ok((Op::FMul, l, r)),
            Expr::FDiv(l, r, _) => Ok((Op::FDiv, l, r)),
            Expr::Eq(l, r, _) => Ok((Op::Eq, l, r)),
            Expr::Ne(l, r, _) => Ok((Op::Ne, l, r)),
            Expr::Lt(l, r, _) => Ok((Op::Lt, l, r)),
            Expr::Le(l, r, _) => Ok((Op::Le, l, r)),
            Expr::Gt(l, r, _) => Ok((Op::Gt, l, r)),
            Expr::Ge(l, r, _) => Ok((Op::Ge, l, r)),
            Expr::FEq(l, r, _) => Ok((Op::FEq, l, r)),
            Expr::FNe(l, r, _) => Ok((Op::FNe, l, r)),
            Expr::FLt(l, r, _) => Ok((Op::FLt, l, r)),
            Expr::FLe(l, r, _) => Ok((Op::FLe, l, r)),
            Expr::FGt(l, r, _) => Ok((Op::FGt, l, r)),
            Expr::FGe(l, r, _) => Ok((Op::FGe, l, r)),
            Expr::Xor(l, r, _) => Ok((Op::Xor, l, r)),
            Expr::LAnd(l, r, _) => Ok((Op::LAnd, l, r)),
            Expr::LOr(l, r, _) => Ok((Op::LOr, l, r)),
            Expr::StrCat(l, r, _) => Ok((Op::StrCat, l, r)),
            _ => Err(CodeGenError::UnsupportedOperation {
                message: "not a binary operation".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;
    use std::collections::HashMap;

    fn compile_consts(src: &str) -> HashMap<String, IRConst> {
        let lexer = Lexer::new(src);
        let mut parser = Parser::new(lexer);
        let program = parser.parse().expect("parse failed");
        let mut irgen = IRGen::new();
        irgen.compile(program).expect("compile failed");
        irgen
            .globals
            .into_iter()
            .map(|(k, (c, _))| (k, c))
            .collect()
    }

    #[test]
    fn const_via_pure_function() {
        let src = "fun(pure) fact(n: int): int {
    if n < 2 { return 1 }
    return n * fact(n - 1)
}
cst FACT: int = fact(10)
cst FIB: int = fib(20)
fun(pure) fib(n: int): int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}
";
        let consts = compile_consts(src);
        assert_eq!(consts.get("FACT"), Some(&IRConst::Int(3628800)));
        assert_eq!(consts.get("FIB"), Some(&IRConst::Int(6765)));
    }
}
