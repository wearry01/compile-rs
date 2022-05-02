use super::ast::*;
use super::config::{
  Value as SymValue,
  Config,
};
use super::value::Value;
use super::value::Initializer;
use super::FrontendError;

use koopa::ir::builder_traits::*;
use koopa::ir::{
  Type,
  TypeKind,
  // Value as IrValue,
  BinaryOp as IrBinaryOp,
};

pub type Result<T> = std::result::Result<T, FrontendError>;

pub trait ProgramGen<'ast> {
  type Out;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out>;
}

impl<'ast> ProgramGen<'ast> for CompUnit {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    // generate decl for lib_functions
    config.new_decl("getint", vec![], Type::get_i32())?;
    config.new_decl("getch", vec![], Type::get_i32())?;
    config.new_decl(
      "getarray", 
      vec![Type::get_pointer(Type::get_i32())], 
      Type::get_i32()
    )?;
    config.new_decl("putint", vec![Type::get_i32()], Type::get_unit())?;
    config.new_decl("putch", vec![Type::get_i32()], Type::get_unit())?;
    config.new_decl(
      "putarray", 
      vec![
        Type::get_i32(),
        Type::get_pointer(Type::get_i32())
      ], 
      Type::get_unit()
    )?;
    config.new_decl("starttime", vec![], Type::get_unit())?;
    config.new_decl("stoptime", vec![], Type::get_unit())?;

    for func in &self.global_def {
      match func {
        GlobalDef::Decl(decl) => decl.generate(config)?,
        GlobalDef::FuncDef(funcdef) => funcdef.generate(config)?,
      }
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for FuncDef {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let func_ty = self.func_type.generate(config)?;
    let params_ty = self.params.iter().map(|p|
      p.generate(config)
    ).collect::<Result<Vec<_>>>()?;

    config.enter_func(
      &self.ident,
      params_ty,
      func_ty,
    )?;

    // generate symbol for function args
    let p_params = config.func_mut().params().to_owned();
    for (p, v) in self.params.iter().zip(p_params) {
      let ty = config.value_ty(v);
      let alloc = config.new_value_builder().alloc(ty);
      config.insert_instr(alloc);
      config.set_name(alloc, &p.ident);
      let store = config.new_value_builder().store(v, alloc);
      config.insert_instr(store);
      config.new_value(&p.ident, SymValue::Value(alloc))?;
    }

    self.block.generate(config)?;

    config.leave_func();
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for FuncType {
  type Out = Type;
  fn generate(&'ast self, _config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Int => Ok(Type::get_i32()),
      Self::Void => Ok(Type::get_unit()),
    }
  }
}

impl<'ast> ProgramGen<'ast> for FuncFParam {
  type Out = Type;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    if let Some(dims) = &self.dims { // array args
      Ok(Type::get_pointer(dims.generate(config)?))
    } else {
      Ok(Type::get_i32())
    }
  }
}

impl<'ast> ProgramGen<'ast> for Block {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    for blockitem in &self.item {
      blockitem.generate(config)?;
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for BlockItem {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Decl(decl) => decl.generate(config)?,
      Self::Stmt(stmt) => stmt.generate(config)?,
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for Decl {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::ConstDecl(c) => c.generate(config),
      Self::VarDecl(v) => v.generate(config),
    }
  }
}

impl<'ast> ProgramGen<'ast> for ConstDecl {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    for constdef in &self.item {
      constdef.generate(config)?;
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for ConstDef {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let ty = self.dims.generate(config)?;
    let init = self.initial.generate(config)?.fit(&ty)?;

    if ty.is_i32() { // create new imm number
      match init {
        Initializer::Const(i) => {
          config.new_value(&self.ident, SymValue::Const(i))?
        },
        _ => unreachable!(),
      }
    } else {
      let value = if config.is_global() {
        let init = init.as_const(config)?;
        let value = config.global_new_value_builder().global_alloc(init);
        value
      } else {
        let value = config.new_value_builder().alloc(ty);
        config.insert_instr(value);
        init.as_store(config, value);
        value
      };
      config.set_name(value, &self.ident);
      config.new_value(&self.ident, SymValue::Value(value))?;
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for ConstInitVal {
  type Out = Initializer;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    Ok(match self {
      Self::Exp(constexp) => Initializer::Const(constexp.generate(config)?),
      Self::List(list) => Initializer::List(
        list.iter().map(|i| i.generate(config)).collect::<Result<_>>()?
      ),
    })
  }
}

impl<'ast> ProgramGen<'ast> for VarDecl {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    for vardef in &self.item {
      vardef.generate(config)?;
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for VarDef {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let ty = self.dims.generate(config)?;
    let alloc = match &self.initial {
      None => {
        if config.is_global() {
          let init = config.global_new_value_builder().zero_init(ty);
          config.global_new_value_builder().global_alloc(init)
        } else {
          let alloc = config.new_value_builder().alloc(ty);
          config.insert_instr(alloc);
          alloc
        }
      },
      Some(init) => {
        let init = init.generate(config)?.fit(&ty)?;

        if config.is_global() {
          let init = init.as_const(config)?;
          config.global_new_value_builder().global_alloc(init)
        } else {
          let alloc = config.new_value_builder().alloc(ty);
          config.insert_instr(alloc);
          init.as_store(config, alloc);
          alloc
        }
      }
    };
    config.set_name(alloc, &self.ident);
    config.new_value(&self.ident, SymValue::Value(alloc))
  }
}

impl<'ast> ProgramGen<'ast> for InitVal {
  type Out = Initializer;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let init = match self {
      Self::Exp(exp) => {
        if config.is_global() {
          Initializer::Const(exp.eval(config).ok_or(FrontendError::EvalConstExpFail)?)
        } else {
          Initializer::Value(exp.generate(config)?.as_int(config)?)
        }
      }
      Self::List(list) => Initializer::List( 
        list.iter().map(|v| v.generate(config)).collect::<Result<Vec<_>>>()?
      ),
    };
    Ok(init)
  }
}

// get array type
impl<'ast> ProgramGen<'ast> for Vec<ConstExp> {
  type Out = Type;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let mut ty = Type::get_i32();
    for exp in self.iter().rev() {
      let len = exp.generate(config)?;
      if len < 1 {
        return Err(FrontendError::InvalidInitializer);
      }
      ty = Type::get_array(ty, len as usize);
    }
    Ok(ty)
  }
}

impl<'ast> ProgramGen<'ast> for Stmt {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Assign(assign) => assign.generate(config)?,
      Self::ExpStmt(expstmt) => expstmt.generate(config)?,
      Self::Block(block) => {
        config.scope_in();
        block.generate(config)?;
        config.scope_out();
      },
      Self::If(if_stmt) => if_stmt.generate(config)?,
      Self::While(while_stmt) => while_stmt.generate(config)?,
      Self::Break(break_stmt) => break_stmt.generate(config)?,
      Self::Continue(continue_stmt) => continue_stmt.generate(config)?,
      Self::Return(exp) => {
        if let Some(exp) = exp {
          let value = exp.generate(config)?.as_int(config)?;
          let ret_val = config.ret_val().unwrap();
          let store = config.new_value_builder().store(value, ret_val);
          config.insert_instr(store);
        }
        let end = config.end();
        let jump = config.new_value_builder().jump(end);
        config.insert_instr(jump);
        let skipped = config.new_bb("%skipped".into());
        config.set_bb(skipped);
      },
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for Assign {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let lval = self.lval.generate(config)?.as_ptr()?;
    let exp = self.exp.generate(config)?.as_val(config)?;
    let store = config.new_value_builder().store(exp, lval);
    config.insert_instr(store);
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for ExpStmt {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    if let Some(e) = &self.exp {
      e.generate(config)?;
    }
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for If {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let cond = self.cond.generate(config)?.as_int(config)?;
    let then_block = config.new_bb("%then".into());

    let end_if;
    if let Some(else_stmt) = &self.else_stmt {
      let else_block = config.new_bb("%else".into());
      end_if = config.new_bb("%endif".into());
      let branch = config.new_value_builder().branch(cond, then_block, else_block);
      config.insert_instr(branch);

      config.set_bb(else_block);
      else_stmt.generate(config)?;
      let jump = config.new_value_builder().jump(end_if);
      config.insert_instr(jump);
    } else {
      end_if = config.new_bb("%endif".into());
      let branch = config.new_value_builder().branch(cond, then_block, end_if);
      config.insert_instr(branch);
    }

    config.set_bb(then_block);
    self.then_stmt.generate(config)?;
    let jump = config.new_value_builder().jump(end_if);
    config.insert_instr(jump);

    config.set_bb(end_if);
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for While {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let bb_entry = config.new_bb("%loop_entry".into());
    let bb_body = config.new_bb("%loop_body".into());
    let bb_end = config.new_bb("%loop_end".into());
    let jump = config.new_value_builder().jump(bb_entry);
    config.insert_instr(jump);

    config.set_bb(bb_entry);
    let cond = self.cond.generate(config)?.as_int(config)?;
    let branch = config.new_value_builder().branch(cond, bb_body, bb_end);
    config.insert_instr(branch);

    config.while_in(bb_entry, bb_end);
    config.set_bb(bb_body);
    self.stmt.generate(config)?;
    let jump = config.new_value_builder().jump(bb_entry);
    config.insert_instr(jump);
    config.while_out();

    config.set_bb(bb_end);
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for Break {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let dest = config.break_bb();
    let jump = config.new_value_builder().jump(dest);
    config.insert_instr(jump);
    let skipped = config.new_bb("%skipped".into());
    config.set_bb(skipped);
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for Continue {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let dest = config.continue_bb();
    let jump = config.new_value_builder().jump(dest);
    config.insert_instr(jump);
    let skipped = config.new_bb("%skipped".into());
    config.set_bb(skipped);
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for Exp {
  type Out = Value;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Number(v) => Ok(Value::Int(
        if config.is_global() {
          config.global_new_value_builder().integer(*v)
        } else {
          config.new_value_builder().integer(*v)
        }
      )),
      Self::LVal(lval) => lval.generate(config),
      Self::UnaryExp(uop, exp) => {
        let value = exp.generate(config)?.as_int(config)?;
        let zero = config.new_value_builder().integer(0);
        let binary = match uop {
          UnaryOp::Neg => config.new_value_builder().binary(IrBinaryOp::Sub, zero, value),
          UnaryOp::Not => config.new_value_builder().binary(IrBinaryOp::Eq, zero, value),
        };
        config.insert_instr(binary);
        Ok(Value::Int(binary))
      },
      Self::BinaryExp(lhs, op, rhs) => {
        match op {
          BinaryOp::And => {
            let result = config.new_value_builder().alloc(Type::get_i32());
            config.insert_instr(result);
            {
              let zero = config.new_value_builder().integer(0);
              let store = config.new_value_builder().store(zero, result);
              config.insert_instr(store);
            }
            let lval = lhs.generate(config)?.as_int(config)?;
            let reval = config.new_bb("%reval".into());
            let short_path = config.new_bb("%short_path".into());
            let branch = config.new_value_builder().branch(lval, reval, short_path);
            config.insert_instr(branch);

            config.set_bb(reval);
            let rval = rhs.generate(config)?.as_int(config)?;
            let zero = config.new_value_builder().integer(0);
            let rval = config.new_value_builder().binary(IrBinaryOp::NotEq, zero, rval);
            config.insert_instr(rval);
            let store = config.new_value_builder().store(rval, result);
            config.insert_instr(store);

            let jump = config.new_value_builder().jump(short_path);
            config.insert_instr(jump);

            config.set_bb(short_path);
            let load = config.new_value_builder().load(result);
            config.insert_instr(load);
            Ok(Value::Int(load))
          },
          BinaryOp::Or => {
            let result = config.new_value_builder().alloc(Type::get_i32());
            config.insert_instr(result);
            {
              let one = config.new_value_builder().integer(1);
              let store = config.new_value_builder().store(one, result);
              config.insert_instr(store);
            }
            let lval = lhs.generate(config)?.as_int(config)?;
            let reval = config.new_bb("%reval".into());
            let short_path = config.new_bb("%short_path".into());
            let branch = config.new_value_builder().branch(lval, short_path, reval);
            config.insert_instr(branch);

            config.set_bb(reval);
            let rval = rhs.generate(config)?.as_int(config)?;
            let zero = config.new_value_builder().integer(0);
            let rval = config.new_value_builder().binary(IrBinaryOp::NotEq, zero, rval);
            config.insert_instr(rval);
            let store = config.new_value_builder().store(rval, result);
            config.insert_instr(store);

            let jump = config.new_value_builder().jump(short_path);
            config.insert_instr(jump);

            config.set_bb(short_path);
            let load = config.new_value_builder().load(result);
            config.insert_instr(load);
            Ok(Value::Int(load))
          },
          other => {
            let lval = lhs.generate(config)?.as_int(config)?;
            let rval = rhs.generate(config)?.as_int(config)?;
            let bop = other.generate(config)?;
            let binary = config.new_value_builder().binary(bop, lval, rval);
            config.insert_instr(binary);
            Ok(Value::Int(binary))
          }
        }
      },
      Self::FuncCall(ident, params) => {
        let func = config.get_func(ident)?;
        let args = params.iter().map(|p| 
          p.generate(config)?.as_val(config)
        ).collect::<Result<Vec<_>>>()?;
        let call = config.new_value_builder().call(func, args);
        config.insert_instr(call);
        
        if config.is_void(func) {
          Ok(Value::Nav)
        } else {
          Ok(Value::Int(call))
        }
      }
    }
  }
}

impl<'ast> ProgramGen<'ast> for ConstExp {
  type Out = i32;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    self.exp.eval(config).ok_or(FrontendError::EvalConstExpFail)
  }
}

impl<'ast> ProgramGen<'ast> for LVal {
  type Out = Value;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let mut value = match config.get_value(&self.ident)? {
      SymValue::Value(v) => v,
      SymValue::Const(v) => {
        return if self.indices.is_empty() {
          Ok(Value::Int(config.new_value_builder().integer(v)))
        } else {
          Err(FrontendError::InvalidValueType)
        };
      }
    };

    let mut arr_args = false;
    let mut dims = match config.value_ty(value).kind() {
      TypeKind::Pointer(base0) => {
        let mut ty = base0;
        let mut dims = 0;

        loop {
          ty = match ty.kind() {
            TypeKind::Array(base1, _) => base1,
            TypeKind::Pointer(base1) => {
              arr_args = true;
              base1
            },
            _ => break dims,
          };
          dims += 1;
        }
      },
      _ => 0,
    };

    if arr_args {
      value = config.new_value_builder().load(value);
      config.insert_instr(value);
    }

    for (i, idx) in self.indices.iter().enumerate() {
      dims -= 1;
      if dims < 0 {
        return Err(FrontendError::InvalidValueType);
      }

      let idx = idx.generate(config)?.as_int(config)?;
      value = if arr_args && i == 0 {
        config.new_value_builder().get_ptr(value, idx)
      } else {
        config.new_value_builder().get_elem_ptr(value, idx)
      };
      config.insert_instr(value);
    }

    if dims == 0 {
      Ok(Value::Ptr(value))
    } else {
      if !arr_args || !self.indices.is_empty() {
        let zero = config.new_value_builder().integer(0);
        value = config.new_value_builder().get_elem_ptr(value, zero);
        config.insert_instr(value);
      }
      Ok(Value::APtr(value))
    }
  }
}

impl<'ast> ProgramGen<'ast> for BinaryOp {
  type Out = IrBinaryOp;
  fn generate(&'ast self, _config: &mut Config<'ast>) -> Result<Self::Out> {
    Ok(match self {
      Self::Mul => IrBinaryOp::Mul,
      Self::Div => IrBinaryOp::Div,
      Self::Mod => IrBinaryOp::Mod,
      Self::Add => IrBinaryOp::Add,
      Self::Sub => IrBinaryOp::Sub,
      Self::Lt => IrBinaryOp::Lt,
      Self::Le => IrBinaryOp::Le,
      Self::Gt => IrBinaryOp::Gt,
      Self::Ge => IrBinaryOp::Ge,
      Self::Eq => IrBinaryOp::Eq,
      Self::Neq => IrBinaryOp::NotEq,
      Self::And => IrBinaryOp::And,
      Self::Or => IrBinaryOp::Or,
    })
  }
}