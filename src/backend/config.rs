use std::fs::File;
use std::io::{Result, Write};
use std::collections::HashMap;
use koopa::ir::{
  Program,
  Function,
  Value,
  ValueKind,
  TypeKind,
  BasicBlock,
  FunctionData,
};

use koopa::ir::entities::ValueData;

use super::format::Format;

pub struct Config<'p> {
  program: &'p Program,
  cur_func: Option<Function>,
  value_table: HashMap<Value, String>,
  alloc_size: (usize, usize, usize), // ra + local + args
  alloc_table: HashMap<*const ValueData, (usize, bool)>,
  bbs_id: usize,
  bbs_table: HashMap<BasicBlock, String>
}

impl<'p> Config<'p> {
  pub fn new(p: &'p Program) -> Self {
    Self {
      program: p,
      cur_func: None,
      value_table: HashMap::new(),
      alloc_size: (0, 0, 0),
      alloc_table: HashMap::new(),
      bbs_id: 0,
      bbs_table: HashMap::new(),
    }
  }

  pub fn program(&self) -> &'p Program {
    self.program
  }

  pub fn set_func(&mut self, func: Function) {
    self.cur_func = Some(func);
  }

  pub fn value(&self, value: Value) -> &ValueData {
    if let Some(func) = self.cur_func {
      self.program.func(func).dfg().value(value)
    } else {
      unreachable!()
    }
  }

  pub fn stk_frame_size(&self) -> usize {
    let size = self.alloc_size;
    (size.0 + size.1 + size.2 + 15) / 16 * 16
  }

  // Deal Global Values

  pub fn get_value(&self, value: Value) -> &str {
    self.value_table.get(&value).unwrap()
  }

  pub fn new_value(&mut self, value: Value, name: String) {
    self.value_table.insert(value, name);
  }

  // Deal Local Values & Functions

  pub fn sp_offset(&self, value: &ValueData) -> Option<&(usize, bool)> {
    self.alloc_table.get(&(value as *const ValueData))
  }

  pub fn get_bb(&self, bb: &BasicBlock) -> &str {
    self.bbs_table.get(bb).unwrap()
  }

  pub fn prologue(&mut self, file: &mut File, func: &FunctionData) -> Result<()> {

    // Local Allocs
    self.alloc_size = (0, 0, 0);
    self.alloc_table = HashMap::new();

    // Ra & Args
    for value in func.dfg().values().values() {
      if let ValueKind::Call(v) = value.kind() {
        self.alloc_size.0 = 4;
        if v.args().len() * 4 > self.alloc_size.2 + 8 * 4 {
          self.alloc_size.2 = (v.args().len() - 8) * 4;
        }
      }
    }

    // Local Value
    for value in func.dfg().values().values() {
      if value.kind().is_local_inst() && !value.used_by().is_empty() {
        let ptr = self.alloc_size.2 + self.alloc_size.1;
        match value.kind() {
          ValueKind::Alloc(_) => {
            self.alloc_table.insert(value, (ptr, false));
            self.alloc_size.1 += match value.ty().kind() {
              TypeKind::Pointer(p) => p.size(),
              _ => unreachable!(),
            };
          },
          _ => {
            let is_ptr = matches!(value.ty().kind(), TypeKind::Pointer(_));
            self.alloc_table.insert(value, (ptr, is_ptr));
            self.alloc_size.1 += value.ty().size();
          }
        }
      }
    }

    // BBS
    self.bbs_id = 0;
    self.bbs_table = HashMap::new();
    
    for (&bb, bb_t) in func.dfg().bbs() {
      if let Some(bb_name) = bb_t.name() {
        self.bbs_table.insert(bb, format!(
          ".L_{}_{}_{}",
           &func.name()[1..], 
           &bb_name[1..], 
           self.bbs_id
          )
        );
      } else {
        self.bbs_table.insert(bb, format!(
          ".L_{}_{}", 
          &func.name()[1..], 
          self.bbs_id
          )
        );
      }
      self.bbs_id += 1;
    }

    let mut format = Format::new(file);
    let offset = self.stk_frame_size() as i32;
    format.addi("sp", "sp", -offset)?;
    if self.alloc_size.0 > 0 {
      format.sw("ra", "sp", offset - 4)?;
    }
    Ok(())
  }

  pub fn epilogue(&self, file: &mut File) -> Result<()> {
    let mut format = Format::new(file);
    let offset = self.stk_frame_size() as i32;
    if self.alloc_size.0 > 0 {
      format.lw("ra", "sp", offset - 4)?;
    }
    format.addi("sp", "sp", offset)?;
    writeln!(file, "\tret")
  }
}