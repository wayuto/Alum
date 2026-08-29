use super::context::Context;
use super::ir::{IRType, Instruction, Op, Operand};
use crate::compiler::{codegen::CodeGenError, irgen::IRGen, parser::Expr};

impl IRGen {
    pub(super) fn compile_match(
        &mut self,
        target: Box<Expr>,
        branches: Vec<(Expr, Option<Box<Expr>>, Expr)>,
        default: Option<Box<Expr>>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let target = self.compile_expr(*target, ctx)?;
        let end_label = ctx.new_label("end_match");

        let res_ty = branches
            .iter()
            .find_map(|(_, _, b)| self.expr_high_type(b, ctx))
            .map(|t| Context::type_to_ir_type(&t))
            .unwrap_or(IRType::Void);
        let res_tmp = ctx.new_tmp(res_ty);

        let cmp_op = if matches!(
            ctx.get_operand_type(&target, &self.constants)?,
            IRType::String
        ) {
            Op::StrEq
        } else {
            Op::Eq
        };

        let branch_count = branches.len();
        let case_labels: Vec<String> = (0..branch_count).map(|_| ctx.new_label("case")).collect();
        let test_labels: Vec<String> = (0..branch_count)
            .map(|_| ctx.new_label("case_test"))
            .collect();
        let default_label = ctx.new_label("match_default");
        let bodies: Vec<Expr> = branches.iter().map(|(_, _, b)| b.clone()).collect();

        for (case_idx, (case, guard, _body)) in branches.into_iter().enumerate() {
            let next_test = if case_idx + 1 < branch_count {
                test_labels[case_idx + 1].clone()
            } else {
                default_label.clone()
            };
            let case_label = Operand::Label(case_labels[case_idx].clone());

            ctx.instructions.push(Instruction {
                op: Op::Label(test_labels[case_idx].clone()),
                dst: None,
                src1: None,
                src2: None,
            });

            let cond = if let Expr::Range(lo, hi, _) = case {
                if !matches!(ctx.get_operand_type(&target, &self.constants)?, IRType::Int) {
                    return Err(CodeGenError::TypeError {
                        message: "range pattern requires an int match target".to_string(),
                    });
                }
                let lo_op = self.compile_expr(*lo, ctx)?;
                let hi_op = self.compile_expr(*hi, ctx)?;
                let ge_tmp = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: Op::Ge,
                    dst: Some(ge_tmp.clone()),
                    src1: Some(target.clone()),
                    src2: Some(lo_op),
                });
                let lt_tmp = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: Op::Lt,
                    dst: Some(lt_tmp.clone()),
                    src1: Some(target.clone()),
                    src2: Some(hi_op),
                });
                let and_tmp = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: Op::And,
                    dst: Some(and_tmp.clone()),
                    src1: Some(ge_tmp),
                    src2: Some(lt_tmp),
                });
                and_tmp
            } else {
                let case_op = self.compile_expr(case, ctx)?;
                let cond = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: cmp_op.clone(),
                    dst: Some(cond.clone()),
                    src1: Some(target.clone()),
                    src2: Some(case_op),
                });
                cond
            };

            match guard {
                None => {
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfTrue,
                        dst: None,
                        src1: Some(cond),
                        src2: Some(case_label),
                    });
                }
                Some(guard) => {
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfFalse,
                        dst: None,
                        src1: Some(cond),
                        src2: Some(Operand::Label(next_test)),
                    });
                    let moved_before = ctx.moved.clone();
                    let guard_op = self.compile_expr(*guard, ctx)?;
                    ctx.moved = moved_before;
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfTrue,
                        dst: None,
                        src1: Some(guard_op),
                        src2: Some(case_label),
                    });
                }
            }
        }

        ctx.instructions.push(Instruction {
            op: Op::Label(default_label),
            dst: None,
            src1: None,
            src2: None,
        });
        if let Some(d) = default {
            self.compile_scoped_value(*d, &res_tmp, ctx)?;
        } else {
            let zero_idx = self.get_const_index(super::ir::IRConst::Int(0));
            ctx.instructions.push(Instruction {
                op: Op::Move,
                dst: Some(res_tmp.clone()),
                src1: Some(Operand::ConstIdx(zero_idx)),
                src2: None,
            });
        }

        ctx.instructions.push(Instruction {
            op: Op::Jump,
            dst: None,
            src1: Some(Operand::Label(end_label.clone())),
            src2: None,
        });

        for (case_idx, body) in bodies.into_iter().enumerate() {
            ctx.instructions.push(Instruction {
                op: Op::Label(case_labels[case_idx].clone()),
                dst: None,
                src1: None,
                src2: None,
            });
            self.compile_scoped_value(body, &res_tmp, ctx)?;
            ctx.instructions.push(Instruction {
                op: Op::Jump,
                dst: None,
                src1: Some(Operand::Label(end_label.clone())),
                src2: None,
            });
        }
        ctx.instructions.push(Instruction {
            op: Op::Label(end_label),
            dst: None,
            src1: None,
            src2: None,
        });
        Ok(res_tmp)
    }
}
