use super::gen::Result;
use super::FrontendError;

use std::collections::HashMap;
use koopa::ir::builder_traits::*;
use koopa::ir::builder::{LocalBuilder};
use koopa::ir::{
  Type,
  Program,
  BasicBlock, 
  FunctionData,
  Value as IrValue,
  Function as IrFunction, 
};

#[derive(Clone)]
pub enum Value {
  Const(i32),
  Value(IrValue),
}

// information about (current) function
pub struct Function {
  ident: IrFunction,
  entry: BasicBlock,
  ret_val: Option<IrValue>,
}

pub struct Config<'p> {
  program: &'p mut Program,
  pub function: Option<Function>, // None for global config
  pub vardef: Vec<HashMap<&'p str, Value>>, // now a symbol table, TODO: implement scope
}

impl<'p> Config<'p> {

  pub fn new(program: &'p mut Program) -> Self {
    Self {
      program: program,
      function: None,
      vardef: vec![HashMap::new()],
    }
  }

  pub fn ret_val(&self) -> Option<IrValue> {
    return self.function.as_ref().unwrap().ret_val.clone();
  }

  // function begin
  pub fn enter_func(
    &mut self,
    name: String,
    params: Vec<Type>,
    ret_ty: Type,
  ) {
    let mut func_data = FunctionData::new(name, params, ret_ty.clone());
    let entry = func_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    func_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();
    let ret_val = {
      if ret_ty == Type::get_i32() {
        let alloc = func_data.dfg_mut().new_value().alloc(Type::get_i32());
        func_data.dfg_mut().set_value_name(alloc, Some("%ret".into()));
        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc).unwrap();
        Some(alloc)
      } else {
        None
      }
    };
    let ident = self.program.new_func(func_data);
    self.function = Some(Function { ident, entry, ret_val });
  }

  // return before leave
  pub fn leave_func(&mut self) {
    let ret_val = self.ret_val();
    let ret_val = self.new_value_builder().load(ret_val.unwrap());
    self.insert_instr(ret_val);
    let instr = self.new_value_builder().ret(Some(ret_val));
    self.insert_instr(instr);
  }

  // insert new value definition into symbol table
  pub fn new_value(&mut self, id: &'p str, value: Value) -> Result<()> {
    let symbol_table = self.vardef.last_mut().unwrap();
    if symbol_table.contains_key(id) {
      Err(FrontendError::MultiDef(id.into()))
    } else {
      symbol_table.insert(id, value);
      Ok(())
    }
  }

  // retrieve an value by ident 
  pub fn get_value(&self, id: &str) -> Option<Value> {
    if let Some(v) = self.vardef[0].get(id) {
      Some(v.clone())
    } else {
      None
    }
  }

  // generate builder for current function
  pub fn new_value_builder<'c>(&'c mut self) -> LocalBuilder<'c> {
    if let Some(f) = &self.function {
      let value = self.program.func_mut(f.ident).dfg_mut().new_value();
      value
    } else {
      unreachable!()
    }
  }

  /*
  // create a new basic block in current function
  pub fn new_bb(&mut self, name: String) -> BasicBlock {
    if let Some(f) = &self.function {
      let bb = self.program.func_mut(f.ident).dfg_mut().new_bb().basic_block(Some(name));
      self.program.func_mut(f.ident).layout_mut().bbs_mut().push_key_back(bb).unwrap();
      bb
    } else {
      unreachable!()
    }
  }
  */

  // insert instruction to current basic block
  pub fn insert_instr(&mut self, instr: IrValue) {
    if let Some(f) = &self.function {
      self.program.func_mut(f.ident).layout_mut().bb_mut(f.entry).insts_mut().push_key_back(instr).unwrap();
    }
  }
}