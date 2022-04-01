/*
  gen 
  generate in-memory koopa ir program from ast
*/

use super::{FrontendError};
use super::ast::*;
use koopa::ir::builder_traits::*;
use koopa::ir::{Program, FunctionData, Type};

type Result<T> = std::result::Result<T, FrontendError>;

pub trait ProgramGen<'ast> {
  type Out;
  fn generate(&'ast self, program: &mut Program) -> Result<Self::Out>;
}

impl<'ast> ProgramGen<'ast> for CompUnit {
  type Out = ();
  fn generate(&'ast self, program: &mut Program) -> Result<Self::Out> {
    self.func_def.generate(program)?;
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for FuncDef {
  type Out = ();
  fn generate(&'ast self, program: &mut Program) -> Result<Self::Out> {
    let mut func_data = FunctionData::new(
      format!("@{}", self.ident),
      vec![],
      self.func_type.generate(program)?,
    );

    // entry/end basic block
    let entry = func_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));

    func_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();

    // alloc return value
    if matches!(self.func_type, FuncType::Int) {

    }

    let func = program.new_func(func_data);

    // generate return instruction
    let ret_v = self.block.generate(program)?;
    let ret_v = program.func_mut(func).dfg_mut().new_value().integer(ret_v);
    let ret_i = program.func_mut(func).dfg_mut().new_value().ret(Some(ret_v));

    program.func_mut(func).layout_mut().bb_mut(entry).insts_mut().push_key_back(ret_i).unwrap();
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for FuncType {
  type Out = Type;
  fn generate(&'ast self, _program: &mut Program) -> Result<Self::Out> {
    match self {
      Self::Int => Ok(Type::get_i32()),
    }
  }
}

impl<'ast> ProgramGen<'ast> for Block {
  type Out = i32;
  fn generate(&'ast self, _program: &mut Program) -> Result<Self::Out> {
    Ok(self.stmt.generate(_program)?)
  }
}

impl<'ast> ProgramGen<'ast> for Stmt {
  type Out = i32;
  fn generate(&'ast self, _program: &mut Program) -> Result<Self::Out> {
    Ok(self.ret_value)
  }
}