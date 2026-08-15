use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};

impl IRGen {
    pub(super) fn compile_array_literal(
        &mut self,
        elements: Vec<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let mut compiled = Vec::new();
        for e in elements.iter() {
            compiled.push(self.compile_expr(e.clone(), ctx)?);
        }

        let ir_const = IRConst::Array(compiled.clone());
        let ir_type = IRType::Array;

        let res_tmp = ctx.new_tmp(ir_type.clone());
        let const_idx = self.get_const_index(ir_const);
        match ir_type {
            IRType::Float => ctx.instructions.push(Instruction {
                op: Op::FMove,
                dst: Some(res_tmp.clone()),
                src1: Some(Operand::ConstIdx(const_idx)),
                src2: None,
            }),
            _ => ctx.instructions.push(Instruction {
                op: Op::Move,
                dst: Some(res_tmp.clone()),
                src1: Some(Operand::ConstIdx(const_idx)),
                src2: None,
            }),
        }
        Ok(res_tmp)
    }

    pub(super) fn compile_array_fill(
        &mut self,
        typ: Type,
        len: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let len_op = self.compile_expr(*len, ctx)?;
        let elem_size = match &typ {
            Type::Primitive(Primitive::Boolean) => 1i64,
            _ => 8i64,
        };
        let ptr_tmp = ctx.new_tmp(IRType::Int);
        if let Operand::ConstIdx(n) = &len_op {
            let n_val = if let IRConst::Int(v) = &self.constants[*n] {
                *v
            } else {
                elem_size
            };
            let total_size = n_val * elem_size + 8;
            let total_size_idx = self.get_const_index(IRConst::Int(total_size));
            let len_idx = self.get_const_index(IRConst::Int(n_val));
            ctx.instructions.push(Instruction {
                op: Op::Malloc,
                dst: Some(ptr_tmp.clone()),
                src1: Some(Operand::ConstIdx(total_size_idx)),
                src2: None,
            });
            let zero_idx = self.get_const_index(IRConst::Int(0));
            ctx.instructions.push(Instruction {
                op: Op::StoreAt,
                dst: Some(ptr_tmp.clone()),
                src1: Some(Operand::ConstIdx(zero_idx)),
                src2: Some(Operand::ConstIdx(len_idx)),
            });
        } else {
            let esize_idx = self.get_const_index(IRConst::Int(elem_size));
            let byte_len_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::Mul,
                dst: Some(byte_len_tmp.clone()),
                src1: Some(len_op.clone()),
                src2: Some(Operand::ConstIdx(esize_idx)),
            });
            let eight_idx = self.get_const_index(IRConst::Int(8));
            let total_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::Add,
                dst: Some(total_tmp.clone()),
                src1: Some(byte_len_tmp),
                src2: Some(Operand::ConstIdx(eight_idx)),
            });
            ctx.instructions.push(Instruction {
                op: Op::Malloc,
                dst: Some(ptr_tmp.clone()),
                src1: Some(total_tmp),
                src2: None,
            });
            let zero_idx = self.get_const_index(IRConst::Int(0));
            ctx.instructions.push(Instruction {
                op: Op::StoreAt,
                dst: Some(ptr_tmp.clone()),
                src1: Some(Operand::ConstIdx(zero_idx)),
                src2: Some(len_op),
            });
        }
        Ok(ptr_tmp)
    }

    pub(super) fn compile_range(
        &mut self,
        start: Box<Expr>,
        end: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let start_op = self.compile_expr(*start, ctx)?;
        let end_op = self.compile_expr(*end, ctx)?;
        let res_tmp = ctx.new_tmp(IRType::Array);
        ctx.instructions.push(Instruction {
            op: Op::Range,
            dst: Some(res_tmp.clone()),
            src1: Some(start_op),
            src2: Some(end_op),
        });
        Ok(res_tmp)
    }
}
