use super::FrontendError;

use super::gen::Result;
use super::config::Config;

use koopa::ir::builder_traits::*;
use koopa::ir::{
  Type,
  TypeKind,
  Value as IrValue,
};

pub enum Value {
  Nav, // Not a value
  Int(IrValue), // Integer Value
  Ptr(IrValue), // Pointer Value
  APtr(IrValue), // Array Pointer Value
}

impl Value {
  // convert to right value
  pub fn as_val(self, config: &mut Config) -> Result<IrValue> { 
    match self {
      Self::Nav => Err(FrontendError::InvalidValueType),
      Self::Int(i) => Ok(i),
      Self::Ptr(p) => {
        let load = config.new_value_builder().load(p);
        config.insert_instr(load);
        Ok(load)
      },
      Self::APtr(p) => Ok(p),
    }
  }
  // convert to right integer value
  pub fn as_int(self, config: &mut Config) -> Result<IrValue> {
    match self {
      Self::APtr(_) => Err(FrontendError::InvalidValueType),
      _ => self.as_val(config),
    }
  }
  // concert to left pointer value
  pub fn as_ptr(self) -> Result<IrValue> {
    match self {
      Self::Ptr(p) => Ok(p),
      _ => Err(FrontendError::InvalidValueType),
    }
  }
}

// Array Initializer
#[derive(Clone)]
pub enum Initializer {
  Const(i32),
  Value(IrValue),
  List(Vec<Initializer>),
}

impl Initializer {
  // [d1][d2] -> ((d2, d2), (d1, d1d2))
  fn expand(ty: &Type) -> Vec<(usize, usize)> {
    match ty.kind() {
      TypeKind::Int32 => Vec::new(),
      TypeKind::Array(base, length) => {
        let mut v = Self::expand(base);
        if v.len() == 0 {
          v.push((*length, *length));
        } else {
          let last_length = v.last().unwrap().1;
          v.push((*length, last_length * *length));
        }
        v
      },
      _ => unreachable!(),
    }
  }

  // get 1d array by filling zeros
  fn fill(inits: Vec<Self>, lens: &[(usize, usize)]) -> Result<Self> {
    let mut filled = Vec::new();
    let size = lens.last().unwrap().1; // total number of values
    for init in inits {
      match init {
        Self::List(list) => {
          let mut align:usize = 0;
          let curlen = filled.len();
          while align+1 < lens.len() && curlen % lens[align].0 == 0 {
            align += 1; 
          }

          if align == 0 { // not aligned to last level
            return Err(FrontendError::InvalidInitializer);
          } else {
            match Self::fill(list, &lens[..align])? {
              Self::List(l) => filled.extend(l),
              _ => unreachable!(),
            }
          }
        },
        _ => {
          filled.push(init);
        },
      }
      if filled.len() > size {
        return Err(FrontendError::InvalidInitializer);
      }
    }
    while filled.len() < size {
      filled.push(Self::Const(0));
    }
    Ok(Self::List(filled))
  }

  // reshape array (must be filled already)
  fn reshape(&self, lens: &[(usize, usize)]) -> Self {
    match self {
      Self::List(list) => {
        if lens.len() == 1 {
          assert_eq!(list.len(), lens[0].0);
          self.clone()
        } else {
          let mut reshaped = Vec::new();
          let size = lens.last().unwrap().0;
          let len = list.len() / size;

          for i in 0..size {
            reshaped.push(
              Self::List(
                list[i*len..(i+1)*len].to_vec()
              ).reshape(&lens[0..lens.len()-1])
            );
          }
          Self::List(reshaped)
        }
      },
      _ => unreachable!(),
    }
  }

  pub fn fit(self, ty: &Type) -> Result<Self> {
    let lens = Self::expand(ty);
    if lens.is_empty() {
      match self {
        Self::Value(v) => Ok(Self::Value(v)),
        Self::Const(v) => Ok(Self::Const(v)),
        _ => Err(FrontendError::InvalidInitializer),
      }
    } else {
      match self {
        Self::List(list) => Ok(Self::fill(list, &lens)?.reshape(&lens)),
        _ => Err(FrontendError::InvalidInitializer),
      }
    }
  }

  pub fn as_const(&self, config: &mut Config) -> Result<IrValue> {
    match self {
      Self::Const(i) => if config.is_global() {
        Ok(config.global_new_value_builder().integer(*i))
      } else {
        Ok(config.new_value_builder().integer(*i))
      },
      Self::Value(_) => Err(FrontendError::EvalConstExpFail),
      Self::List(list) => {
        let init = list.into_iter().map(
          |v| v.as_const(config)
        ).collect::<Result<_>>()?;

        Ok(if config.is_global() {
          config.global_new_value_builder().aggregate(init)
        } else {
          config.new_value_builder().aggregate(init)
        })
      }
    }
  }

  pub fn as_store(&self, config: &mut Config, ptr: IrValue) {
    let store = match self {
      Self::Const(i) => {
        let value = config.new_value_builder().integer(*i);
        config.new_value_builder().store(value, ptr)
      },
      Self::Value(value) => config.new_value_builder().store(*value, ptr),
      Self::List(list) => {
        for (i, init) in list.into_iter().enumerate() {
          let idx = config.new_value_builder().integer(i as i32);
          let new_ptr = config.new_value_builder().get_elem_ptr(ptr, idx);
          config.insert_instr(new_ptr);
          init.as_store(config, new_ptr);
        }
        return;
      }
    };
    config.insert_instr(store);
  }
}