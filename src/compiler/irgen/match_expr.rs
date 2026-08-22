use super::context::Context;
use super::ir::{IRType, Instruction, Op, Operand};
use crate::compiler::{codegen::CodeGenError, irgen::IRGen, parser::Expr};

impl IRGen {
    pub(super) fn compile_match(
        &mut self,
        target: Box<Expr>,
        branches: Vec<(Expr, Expr)>,
        default: Option<Box<Expr>>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let target = self.compile_expr(*target, ctx)?;
        let end_label = ctx.new_label("end_match");
        let mut case_labels: Vec<String> = Vec::new();
        let res_tmp = ctx.new_tmp(IRType::Void);

        for _ in branches.clone() {
            case_labels.push(ctx.new_label("case"));
        }

        let mut case_cnt = 0usize;
        // String targets compare by content (strcmp), mirroring how `==` is rewritten.
        let cmp_op = if matches!(
            ctx.get_operand_type(&target, &self.constants)?,
            IRType::String
        ) {
            Op::StrEq
        } else {
            Op::Eq
        };
        for (case, _) in branches.clone() {
            let cond = ctx.new_tmp(IRType::Bool);
            let case = self.compile_expr(case, ctx)?;
            ctx.instructions.push(Instruction {
                op: cmp_op.clone(),
                dst: Some(cond.clone()),
                src1: Some(target.clone()),
                src2: Some(case),
            });
            ctx.instructions.push(Instruction {
                op: Op::JumpIfTrue,
                dst: None,
                src1: Some(cond),
                src2: Some(Operand::Label(
                    case_labels.iter().nth(case_cnt).unwrap().clone(),
                )),
            });
            case_cnt += 1;
        }

        if let Some(d) = default {
            ctx.enter_scope();
            let d_info = self.resource_copy_info(&d, ctx);
            let ret = self.compile_expr(*d, ctx)?;
            let ret = match d_info {
                Some(ty) => self.copy_resource(ctx, ret, &ty)?,
                None => ret,
            };
            ctx.instructions.push(Instruction {
                op: Op::Move,
                dst: Some(res_tmp.clone()),
                src1: Some(ret),
                src2: None,
            });
            self.emit_scope_frees(ctx)?;
            ctx.exit_scope()?;
        }

        ctx.instructions.push(Instruction {
            op: Op::Jump,
            dst: None,
            src1: Some(Operand::Label(end_label.clone())),
            src2: None,
        });

        case_cnt = 0;
        for (_, ret) in branches.clone() {
            ctx.instructions.push(Instruction {
                op: Op::Label(case_labels.iter().nth(case_cnt).unwrap().clone()),
                dst: None,
                src1: None,
                src2: None,
            });
            ctx.enter_scope();
            let ret_info = self.resource_copy_info(&ret, ctx);
            let ret = self.compile_expr(ret, ctx)?;
            let ret = match ret_info {
                Some(ty) => self.copy_resource(ctx, ret, &ty)?,
                None => ret,
            };
            ctx.instructions.push(Instruction {
                op: Op::Move,
                dst: Some(res_tmp.clone()),
                src1: Some(ret),
                src2: None,
            });
            self.emit_scope_frees(ctx)?;
            ctx.exit_scope()?;
            ctx.instructions.push(Instruction {
                op: Op::Jump,
                dst: None,
                src1: Some(Operand::Label(end_label.clone())),
                src2: None,
            });
            case_cnt += 1;
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
