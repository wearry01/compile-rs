use std::fs::File;
use std::io::{Write, Result};

use koopa::ir::{
  Program, FunctionData
};

use koopa::ir::entities::{
  ValueData, ValueKind
};

use koopa::ir::values::{
  Return,
};

pub trait AsmGen {
  type Out;
  fn generate(&self, file: &mut File) -> Result<Self::Out>;
}

impl AsmGen for Program {
  type Out = ();
  fn generate(&self, file: &mut File) -> Result<Self::Out> {
    for &func in self.func_layout() {
      self.func(func).generate(file)?;
    }
    Ok(())
  }
}

impl AsmGen for FunctionData {
  type Out = ();
  fn generate(&self, file: &mut File) -> Result<Self::Out> {
    writeln!(file, "\t.text")?;
    writeln!(file, "\t.globl {}", &self.name()[1..])?;
    writeln!(file, "{}:", &self.name()[1..])?;
    
    for (_bb, bb_t) in self.layout().bbs() {
      for &insts in bb_t.insts().keys() {
        let value_t = self.dfg().value(insts);
        if let ValueKind::Return(v) = value_t.kind() {
          let ret_t = self.dfg().value(v.value().unwrap());
          if let ValueKind::Integer(r) = ret_t.kind() {
            writeln!(file, "\tli a0, {}", r.value())?;
          }
          writeln!(file, "\tret")?;
        }
      }
    }
    writeln!(file)
  }
}

impl AsmGen for ValueData {
  type Out = ();
  fn generate(&self, file: &mut File) -> Result<Self::Out> {
    match self.kind() {
      ValueKind::Return(v) => v.generate(file),
      _ => Ok(()),
    }
  }
}

impl AsmGen for Return {
  type Out = ();
  fn generate(&self, _file: &mut File) -> Result<Self::Out> {
    Ok(())
  }
}