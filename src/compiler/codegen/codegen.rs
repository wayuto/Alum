use super::types::CodeGenError;
use crate::compiler::codegen::asm::*;
use crate::compiler::irgen::ir::{IRProgram, Operand as IROperand};
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use std::mem::take;

pub(super) fn parse_reg(s: &str) -> Reg {
    match s {
        "rax" => Reg::Rax,
        "rbx" => Reg::Rbx,
        "rcx" => Reg::Rcx,
        "rdx" => Reg::Rdx,
        "rsi" => Reg::Rsi,
        "rdi" => Reg::Rdi,
        "r8" => Reg::R8,
        "r9" => Reg::R9,
        "r10" => Reg::R10,
        "r11" => Reg::R11,
        "r15" => Reg::R15,
        "rsp" => Reg::Rsp,
        "rbp" => Reg::Rbp,
        "xmm0" => Reg::Xmm0,
        "xmm1" => Reg::Xmm1,
        "xmm2" => Reg::Xmm2,
        "xmm3" => Reg::Xmm3,
        "xmm4" => Reg::Xmm4,
        "xmm5" => Reg::Xmm5,
        "xmm6" => Reg::Xmm6,
        "xmm7" => Reg::Xmm7,
        "xmm8" => Reg::Xmm8,
        "xmm9" => Reg::Xmm9,
        "xmm10" => Reg::Xmm10,
        "xmm11" => Reg::Xmm11,
        "xmm12" => Reg::Xmm12,
        "xmm13" => Reg::Xmm13,
        "xmm14" => Reg::Xmm14,
        "xmm15" => Reg::Xmm15,
        _ => panic!("unknown register: {}", s),
    }
}

pub(super) fn m_rbp(off: usize) -> Operand {
    Operand::Mem(Mem {
        base: Some(Reg::Rbp),
        index: None,
        scale: 0,
        disp: -(off as i32),
        size: None,
    })
}

pub(super) fn m_rbp_q(off: usize) -> Operand {
    Operand::Mem(Mem {
        base: Some(Reg::Rbp),
        index: None,
        scale: 0,
        disp: -(off as i32),
        size: Some(Size::QWord),
    })
}

pub(super) fn m_base(base: Reg) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: None,
        scale: 0,
        disp: 0,
        size: None,
    })
}

pub(super) fn m_base_disp(base: Reg, disp: i32) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: None,
        scale: 0,
        disp,
        size: None,
    })
}

pub(super) fn m_sib(base: Reg, index: Reg, scale: u8, disp: i32) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: Some(index),
        scale,
        disp,
        size: None,
    })
}

pub(super) fn rel(lbl: String) -> Operand {
    Operand::DataLabel(lbl)
}

pub struct AsmCodeGen {
    pub(super) program: IRProgram,
    pub(super) text_asms: Vec<Asm>,
    pub(super) data_asms: Vec<Asm>,
    pub(super) vars: HashMap<String, usize>,
    pub(super) lbl_cnt: usize,
    pub(super) str_cache: HashMap<String, String>,
    pub(super) flt_cache: HashMap<OrderedFloat<f64>, String>,
    pub(super) arg_reg: Vec<String>,
    pub(super) flt_arg_reg: Vec<String>,
    pub(super) ret_label: String,
    pub(super) regs: HashMap<String, Option<IROperand>>,
    pub(super) curr_fn: String,
    pub(super) curr_flt_reg: usize,
    pub(super) internals: HashSet<String>,
}

impl AsmCodeGen {
    pub fn new(program: IRProgram) -> Self {
        let internals = program
            .functions
            .iter()
            .filter(|f| !f.is_external)
            .map(|f| f.name.clone())
            .collect();
        Self {
            program,
            text_asms: Vec::new(),
            data_asms: Vec::new(),
            vars: HashMap::new(),
            lbl_cnt: 0,
            str_cache: HashMap::new(),
            flt_cache: HashMap::new(),
            arg_reg: vec![
                "rdi".to_string(),
                "rsi".to_string(),
                "rdx".to_string(),
                "rcx".to_string(),
                "r8".to_string(),
                "r9".to_string(),
            ],
            flt_arg_reg: vec![
                "xmm0".to_string(),
                "xmm1".to_string(),
                "xmm2".to_string(),
                "xmm3".to_string(),
                "xmm4".to_string(),
                "xmm5".to_string(),
                "xmm6".to_string(),
                "xmm7".to_string(),
                "xmm8".to_string(),
                "xmm9".to_string(),
                "xmm10".to_string(),
                "xmm11".to_string(),
                "xmm12".to_string(),
                "xmm13".to_string(),
                "xmm14".to_string(),
                "xmm15".to_string(),
            ],
            ret_label: String::new(),
            regs: HashMap::new(),
            curr_fn: String::new(),
            curr_flt_reg: 0,
            internals,
        }
    }

    pub fn compile(&mut self) -> Result<Vec<Asm>, CodeGenError> {
        self.text_asms.push(Asm::Section(Section::Text));
        for func in &self.program.functions {
            if func.is_external {
                let has_internal = self
                    .program
                    .functions
                    .iter()
                    .any(|f| f.name == func.name && !f.is_external);
                if !has_internal {
                    self.text_asms.push(Asm::Extern(func.name.clone()));
                } else {
                }
            }
        }
        self.text_asms.push(Asm::Extern("malloc".to_string()));
        self.text_asms.push(Asm::Extern("strlen".to_string()));
        self.text_asms.push(Asm::Extern("memcpy".to_string()));
        self.text_asms.push(Asm::Extern("strcpy".to_string()));

        self.data_asms.push(Asm::Section(Section::Data));
        self.data_asms.push(Asm::Align(16));
        self.data_asms.push(Asm::Label("neg_mask".to_string()));
        self.data_asms.push(Asm::Dq(vec![0x8000000000000000, 0]));

        for func in take(&mut self.program.functions) {
            self.compile_fn(func)?;
        }

        let defined: Vec<String> = self
            .text_asms
            .iter()
            .filter_map(|a| {
                if let Asm::Global(n) = a {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        self.text_asms.retain(|a| {
            if let Asm::Extern(n) = a {
                !defined.contains(n)
            } else {
                true
            }
        });

        let mut all = Vec::new();
        all.extend(take(&mut self.data_asms));
        all.extend(take(&mut self.text_asms));
        Ok(all)
    }

    pub(super) fn push_text(&mut self, a: Asm) {
        self.text_asms.push(a);
    }

    pub(super) fn push_data(&mut self, a: Asm) {
        self.data_asms.push(a);
    }
}
