/*
TODO

- Fix multi return bug, implement basic blocks
- seperate values of
  - int type
  - ptr type 
  in process of generation
*/

use super::gen::Result;
use super::FrontendError;

use std::collections::HashMap;
use koopa::ir::builder_traits::*;
use koopa::ir::builder::{LocalBuilder};
use koopa::ir::{
  dfg::DataFlowGraph,
  layout::{Layout, InstList, BasicBlockList},
  Type,
  Program,
  BasicBlock, 
  FunctionData,
  Value as IrValue,
  Function as IrFunction, 
};

#[derive(Clone, Copy)]
pub enum Value {
  Const(i32),
  Value(IrValue),
}

// information about (current) function
#[derive(Clone, Copy)]
pub struct Function {
  ident: IrFunction,
  entry: BasicBlock,
  current: BasicBlock,
  end: BasicBlock,
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

  /*
  fn is_global(&self) -> bool {
    self.function.is_none()
  }
  */

  fn dfg_mut(&mut self) -> &mut DataFlowGraph {
    let func = self.function.unwrap();
    self.program.func_mut(func.ident).dfg_mut()
  }

  fn layout_mut(&mut self) -> &mut Layout {
    let func = self.function.unwrap();
    self.program.func_mut(func.ident).layout_mut()
  }

  fn bbs_mut(&mut self) -> &mut BasicBlockList {
    self.layout_mut().bbs_mut()
  }

  fn insts_mut(&mut self) -> &mut InstList {
    let func = self.function.unwrap();
    self.layout_mut().bb_mut(func.current).insts_mut()
  }

  // generate builder for current function
  pub fn new_value_builder<'c>(&'c mut self) -> LocalBuilder<'c> {
    self.dfg_mut().new_value()
  }

  pub fn end(&self) -> BasicBlock {
    self.function.unwrap().end
  }

  pub fn ret_val(&self) -> Option<IrValue> {
    self.function.unwrap().ret_val
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
    let end = func_data.dfg_mut().new_bb().basic_block(Some("%end".into()));
    func_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();
    func_data.layout_mut().bbs_mut().push_key_back(end).unwrap();
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
    self.function = Some(Function { ident, entry, current: entry, end, ret_val });
  }

  // return before leave
  pub fn leave_func(&mut self) {
    // jump to end
    let func = self.function.unwrap();
    let instr = self.new_value_builder().jump(func.end);
    self.insert_instr(instr);
    self.set_bb(func.end);

    // generate ret instr
    let ret_val = self.new_value_builder().load(func.ret_val.unwrap());
    self.insert_instr(ret_val);
    let instr = self.new_value_builder().ret(Some(ret_val));
    self.insert_instr(instr);
  }

  // enter in a new scope
  pub fn scope_in(&mut self) {
    self.vardef.push(HashMap::new());
  }

  // leave out current scope
  pub fn scope_out(&mut self) {
    self.vardef.pop();
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
    let mut index = (self.vardef.len() - 1) as i32;
    while index >= 0 {
      if let Some(v) = self.vardef[index as usize].get(id) {
        return Some(v.clone());
      }
      index -= 1;
    }
    None
  }

  // create a new basic block in current function
  pub fn new_bb(&mut self, name: String) -> BasicBlock {
    let bb = self.dfg_mut().new_bb().basic_block(Some(name));
    self.bbs_mut().push_key_back(bb).unwrap();
    bb
  }

  // set current basic block
  pub fn set_bb(&mut self, bb: BasicBlock) {
    self.function.as_mut().unwrap().current = bb;
  }

  // insert instruction to current basic block
  pub fn insert_instr(&mut self, instr: IrValue) {
    self.insts_mut().push_key_back(instr).unwrap();
  }
}