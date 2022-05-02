use std::fs::File;
use std::io::{Write, Result};

use koopa::ir::{
  Program, FunctionData, Value, TypeKind
};
use koopa::ir::entities::{ ValueData, ValueKind };
use koopa::ir::values::*;

use super::config::Config;
use super::format::Format;
use super::value::Value as AsmValue;

pub trait AsmGen {
  type Out;
  fn generate(&self, file: &mut File, config: &mut Config) -> Result<Self::Out>;
}

impl AsmGen for Program {
  type Out = ();
  fn generate(&self, file: &mut File, config: &mut Config) -> Result<Self::Out> {
    for &value in self.inst_layout() {
      let data = self.borrow_value(value);
      let name = &data.name().as_ref().unwrap()[1..];

      writeln!(file, "\t.data")?;
      writeln!(file, "\t.globl {name}")?;
      writeln!(file, "{name}:")?;
      data.generate(file, config)?;
      writeln!(file)?;

      config.new_value(value, name.into());
    }

    for &func in self.func_layout() {
      config.set_func(func);
      self.func(func).generate(file, config)?;
    }
    Ok(())
  }
}

impl AsmGen for FunctionData {
  type Out = ();
  fn generate(&self, file: &mut File, config: &mut Config) -> Result<Self::Out> {
    if self.layout().entry_bb().is_none() {
      return Ok(())
    }
    let name = &self.name()[1..];
    writeln!(file, "\t.text")?;
    writeln!(file, "\t.globl {name}")?;
    writeln!(file, "{name}:")?;

    config.prologue(file, &self)?;
    
    for (bb, bb_t) in self.layout().bbs() {
      let bb_name = config.get_bb(bb);
      writeln!(file, "{bb_name}:")?;

      for &inst in bb_t.insts().keys() {
        self.dfg().value(inst).generate(file, config)?;
      }
    }
    writeln!(file)
  }
}

impl AsmGen for Value {
  type Out = AsmValue;
  fn generate(&self, _file: &mut File, config: &mut Config) -> Result<Self::Out> {
    if self.is_global() {
      Ok(AsmValue::Global(config.get_value(*self).into()))
    } else {
      let value = config.value(*self);
      let asmvalue = match value.kind() {
        ValueKind::Integer(i) => AsmValue::Const(i.value()),
        ValueKind::FuncArgRef(i) => AsmValue::Arg(i.index()),
        _ => AsmValue::from(config.sp_offset(value)),
      };
      Ok(asmvalue)
    }
  }
}

impl AsmGen for ValueData {
  type Out = ();
  fn generate(&self, file: &mut File, config: &mut Config) -> Result<Self::Out> {

    match self.kind() {
      ValueKind::Integer(v) => writeln!(file, "\t.word {}", v.value())?,
      ValueKind::ZeroInit(_) => writeln!(file, "\t.zero {}", self.ty().size())?,
      ValueKind::Aggregate(v) => {
        for &item in v.elems() {
          config.program().borrow_value(item).generate(file, config)?;
        }
      },
      ValueKind::Load(v) => {
        let src = v.src().generate(file, config)?;
        src.to(file, "t0", 0)?;
        if src.is_ptr() {
          Format::new(file).lw("t0", "t0", 0)?;
        }
        AsmValue::from(config.sp_offset(&self)).load(file, "t0")?;
      },
      ValueKind::Store(v) => {
        v.value().generate(file, config)?.to(file, "t0", config.stk_frame_size())?;
        let dst = v.dest().generate(file, config)?;
        if dst.is_ptr() {
          dst.to(file, "t1", 0)?;
          Format::new(file).sw("t0", "t1", 0)?;
        } else {
          dst.load(file, "t0")?;
        }
      }
      ValueKind::GetPtr(v) => {
        let src = v.src().generate(file, config)?;
        v.index().generate(file, config)?.to(file, "t1", 0)?;
        let size = match self.ty().kind() {
          TypeKind::Pointer(b) => b.size(),
          _ => unreachable!(),
        };
        if src.is_ptr() {
          src.to(file, "t0", 0)?;
        } else {
          src.addr_to(file, "t0")?;
        }
        // t0 + size * t1(idx)
        let mut format = Format::new(file);
        format.muli("t1", "t1", size as i32)?;
        format.bop("add", "t0", "t0", "t1")?;
        AsmValue::from(config.sp_offset(&self)).load(file, "t0")?;
      },
      ValueKind::GetElemPtr(v) => {
        let src = v.src().generate(file, config)?;
        v.index().generate(file, config)?.to(file, "t1", 0)?;
        let size = match self.ty().kind() {
          TypeKind::Pointer(b) => b.size(),
          _ => unreachable!(),
        };
        if src.is_ptr() {
          src.to(file, "t0", 0)?;
        } else {
          src.addr_to(file, "t0")?;
        }
        // t0 + size * t1(idx)
        let mut format = Format::new(file);
        format.muli("t1", "t1", size as i32)?;
        format.bop("add", "t0", "t0", "t1")?;
        AsmValue::from(config.sp_offset(&self)).load(file, "t0")?;
      },
      ValueKind::Binary(v) => {
        v.lhs().generate(file, config)?.to(file, "t0", 0)?;
        v.rhs().generate(file, config)?.to(file, "t1", 0)?;

        let mut format = Format::new(file);
        match v.op() {
          BinaryOp::Add => format.bop("add", "t0", "t0", "t1")?,
          BinaryOp::Sub => format.bop("sub", "t0", "t0", "t1")?,
          BinaryOp::Mul => format.bop("mul", "t0", "t0", "t1")?,
          BinaryOp::Div => format.bop("div", "t0", "t0", "t1")?,
          BinaryOp::Mod => format.bop("rem", "t0", "t0", "t1")?,
          BinaryOp::And => format.bop("and", "t0", "t0", "t1")?,
          BinaryOp::Or => format.bop("or", "t0", "t0", "t1")?,
          BinaryOp::Xor => format.bop("xor", "t0", "t0", "t1")?,
          BinaryOp::Shl => format.bop("sll", "t0", "t0", "t1")?,
          BinaryOp::Shr => format.bop("srl", "t0", "t0", "t1")?,
          BinaryOp::Sar => format.bop("sra", "t0", "t0", "t1")?,
          BinaryOp::Eq => {
            format.bop("xor", "t0", "t0", "t1")?;
            format.uop("seqz", "t0", "t0")?;
          },
          BinaryOp::NotEq => {
            format.bop("xor", "t0", "t0", "t1")?;
            format.uop("snez", "t0", "t0")?;
          },
          BinaryOp::Gt => format.bop("sgt", "t0", "t0", "t1")?,
          BinaryOp::Lt => format.bop("slt", "t0", "t0", "t1")?,
          BinaryOp::Ge => {
            format.bop("slt", "t0", "t0", "t1")?;
            format.uop("seqz", "t0", "t0")?;
          },
          BinaryOp::Le => {
            format.bop("sgt", "t0", "t0", "t1")?;
            format.uop("seqz", "t0", "t0")?;
          },
        }
        AsmValue::from(config.sp_offset(&self)).load(file, "t0")?;
      },
      ValueKind::Branch(v) => {
        v.cond().generate(file, config)?.to(file, "t0", 0)?;
        let mut format = Format::new(file);
        let temp = &format!(".L_TEMP_{}", config.bbs_get_id());
        format.bnez("t0", temp)?;
        format.j(config.get_bb(&v.false_bb()))?;
        format.label(temp)?;
        format.j(config.get_bb(&v.true_bb()))?;
        /*
          br cond TEMP
          j false
        TEMP:
          j true
        */
      }
      ValueKind::Jump(v) => Format::new(file).j(config.get_bb(&v.target()))?,
      ValueKind::Call(v) => {
        for (idx, arg) in v.args().iter().enumerate() {
          arg.generate(file, config)?.to(file, "t0", 0)?;
          AsmValue::Arg(idx).load(file, "t0")?;
        }
        let callee = &config.program().func(v.callee()).name()[1..];
        Format::new(file).call(callee)?;
        if !self.used_by().is_empty() { // otherwise not in symbol table
          AsmValue::from(config.sp_offset(&self)).load(file, "a0")?;
        }
      }
      ValueKind::GlobalAlloc(v) => config.program().borrow_value(v.init()).generate(file, config)?,
      ValueKind::Return(_v) => {
        if let Some(v) = _v.value() {
          v.generate(file, config)?.to(file, "a0", 0)?;
        }
        config.epilogue(file)?;
      }
      _ => {},
    }

    Ok(())
  }
}