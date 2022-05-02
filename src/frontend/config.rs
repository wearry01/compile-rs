use super::gen::Result;
use super::FrontendError;

use std::collections::HashMap;
use koopa::ir::builder_traits::*;
use koopa::ir::builder::{LocalBuilder, GlobalBuilder};
use koopa::ir::{
  dfg::DataFlowGraph,
  layout::{Layout, InstList, BasicBlockList},
  Type,
  TypeKind,
  Program,
  BasicBlock, 
  FunctionData,
  Value as IrValue,
  Function as IrFunction, 
};

// value stored in symbol table
#[derive(Clone, Copy)]
pub enum Value {
  Const(i32),
  Value(IrValue),
}

// information about (current) function
#[derive(Clone, Copy)]
pub struct Function {
  ident: IrFunction,
  current: BasicBlock,
  end: BasicBlock,
  ret_val: Option<IrValue>,
}

pub struct Config<'p> {
  program: &'p mut Program,
  pub function: Option<Function>, // current function info, None for global config
  pub vardef: Vec<HashMap<&'p str, Value>>, // symbol table for var defs
  pub funcdef: HashMap<&'p str, IrFunction>, // symbol table for function defs
  pub while_block: Vec<(BasicBlock, BasicBlock)>, // basic block chains for (while_entry, while_end)
}

// Global API

impl<'p> Config<'p> {
  pub fn new(program: &'p mut Program) -> Self {
    Self {
      program: program,
      function: None,
      vardef: vec![HashMap::new()],
      funcdef: HashMap::new(),
      while_block: vec![],
    }
  }

  pub fn is_global(&self) -> bool {
    self.function.is_none()
  }

  pub fn is_void(&self, func: IrFunction) -> bool {
    match self.program.func(func).ty().kind() {
      TypeKind::Function(_, t) => {
        return t.is_unit();
      },
      _ => unreachable!(),
    }
  }

  pub fn value_ty(&mut self, value: IrValue) -> Type {
    if value.is_global() {
      self.program.borrow_value(value).ty().clone()
    } else {
      self.dfg().value(value).ty().clone()
    }
  }

  pub fn func_mut(&mut self) -> &mut FunctionData {
    let func = self.function.unwrap();
    self.program.func_mut(func.ident)
  }

  fn layout_mut(&mut self) -> &mut Layout { self.func_mut().layout_mut() }

  fn bbs_mut(&mut self) -> &mut BasicBlockList { self.layout_mut().bbs_mut() }

  pub fn dfg(&mut self) -> &DataFlowGraph { self.func_mut().dfg() }

  pub fn dfg_mut(&mut self) -> &mut DataFlowGraph { self.func_mut().dfg_mut() }

  fn insts_mut(&mut self) -> &mut InstList {
    let func = self.function.unwrap();
    self.layout_mut().bb_mut(func.current).insts_mut()
  }

  // generate builder for current function
  pub fn new_value_builder<'c>(&'c mut self) -> LocalBuilder<'c> {
    self.dfg_mut().new_value()
  }

  pub fn global_new_value_builder<'c>(&'c mut self) -> GlobalBuilder<'c> {
    self.program.new_value()
  }

  pub fn set_name(&mut self, value: IrValue, ident: &str) {
    if self.is_global() {
      self.program.set_value_name(value, Some(format!("@{}", ident)));
    } else {
      self.dfg_mut().set_value_name(value, Some(format!("%{}", ident)));
    }
  }
}

// Function

impl<'p> Config<'p> {
  pub fn end(&self) -> BasicBlock {
    self.function.unwrap().end
  }

  pub fn ret_val(&self) -> Option<IrValue> {
    self.function.unwrap().ret_val
  }

  // enter a new function
  pub fn enter_func(
    &mut self,
    name: &'p str,
    params: Vec<Type>,
    ret_ty: Type,
  ) -> Result<()> {
    let mut func_data = FunctionData::new(format!("@{}", name), params, ret_ty.clone());
    let entry = func_data.dfg_mut().new_bb().basic_block(Some("%func_entry".into()));
    let end = func_data.dfg_mut().new_bb().basic_block(Some("%func_end".into()));
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
    self.function = Some(Function { ident, current: entry, end, ret_val });
    self.new_func(name, ident)?;

    // enter in a new scope
    self.scope_in();
    Ok(())
  }

  // return before leave
  pub fn leave_func(&mut self) {
    // jump to end
    let func = self.function.unwrap();
    let jump = self.new_value_builder().jump(func.end);
    self.insert_instr(jump);
    self.set_bb(func.end);

    // generate ret instr
    if let Some(ret_v) = func.ret_val { 
      // Int Type
      let load = self.new_value_builder().load(ret_v);
      self.insert_instr(load);
      let ret = self.new_value_builder().ret(Some(load));
      self.insert_instr(ret);
    } else { 
      // Void Type
      let ret = self.new_value_builder().ret(None); 
      self.insert_instr(ret);
    }

    // leave out current scope
    self.scope_out();

    // set current function as none
    self.function = None;
  }
}

// Scope, Symbol Table & Basic Blocks 

impl<'p> Config<'p> {
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
    let is_global = self.is_global();
    let symbol_table = self.vardef.last_mut().unwrap();
    if symbol_table.contains_key(id) || (is_global && self.funcdef.contains_key(id)) {
      Err(FrontendError::MultiDef(id.into()))
    } else {
      symbol_table.insert(id, value);
      Ok(())
    }
  }

  // retrieve an value by ident 
  pub fn get_value(&self, id: &str) -> Result<Value> {
    let mut index = (self.vardef.len() - 1) as i32;
    while index >= 0 {
      if let Some(v) = self.vardef[index as usize].get(id) {
        return Ok(v.clone());
      }
      index -= 1;
    }
    Err(FrontendError::UndeclaredId(id.into()))
  }

  // insert new function definition into symbol table
  pub fn new_func(&mut self, id: &'p str, func: IrFunction) -> Result<()> {
    if self.funcdef.contains_key(id) || self.vardef.first().unwrap().contains_key(id) {
      Err(FrontendError::MultiDef(id.into()))
    } else {
      self.funcdef.insert(id, func);
      Ok(())
    }
  }

  // create new declaration
  pub fn new_decl(&mut self, id: &'p str, params: Vec<Type>, ret_ty: Type) -> Result<()> {
    let func = self.program.new_func(FunctionData::new(format!("@{}", id), params, ret_ty));
    self.new_func(id, func)
  }
  
  // retrieve a function by ident
  pub fn get_func(&mut self, id: &str) -> Result<IrFunction> {
    self.funcdef.get(id).copied().ok_or(FrontendError::UndeclaredId(id.into()))
  }

  // Methods for while loop blocks
  pub fn while_in(&mut self, bb_entry: BasicBlock, bb_end: BasicBlock) {
    self.while_block.push((bb_entry, bb_end));
  }

  pub fn while_out(&mut self) {
    self.while_block.pop();
  }

  pub fn break_bb(&mut self) -> BasicBlock {
    self.while_block.last().unwrap().1
  }

  pub fn continue_bb(&mut self) -> BasicBlock {
    self.while_block.last().unwrap().0
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