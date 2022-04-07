use super::ast::*;
use super::config::Config;
use super::config::Value;
use super::{FrontendError};
use koopa::ir::builder_traits::*;
use koopa::ir::{
  Type,
  Value as IrValue,
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
    self.func_def.generate(config)?;
    Ok(())
  }
}

impl<'ast> ProgramGen<'ast> for FuncDef {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    // entering current function
    let func_ty = self.func_type.generate(config)?;
    config.enter_func(
      format!("@{}", self.ident),
      vec![],
      func_ty,
    );

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
    let init = self.initial.generate(config)?;
    config.new_value(&self.ident, Value::Const(init)) // create new imm number
  }
}

impl<'ast> ProgramGen<'ast> for ConstInitVal {
  type Out = i32;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Exp(constexp) => constexp.generate(config),
    }
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
    let alloc = config.new_value_builder().alloc(Type::get_i32());
    config.insert_instr(alloc);

    // if initialized, create store instr
    if let Some(init) = &self.initial {
      let init = init.generate(config)?;
      let store = config.new_value_builder().store(init, alloc);
      config.insert_instr(store);
    }
    config.new_value(&self.ident, Value::Value(alloc))
  }
}

impl<'ast> ProgramGen<'ast> for InitVal {
  type Out = IrValue;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    match self {
      Self::Exp(exp) => exp.generate(config),
    }
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
        let value = exp.generate(config)?;
        let ret_val = config.ret_val().unwrap();
        let store = config.new_value_builder().store(value, ret_val);
        config.insert_instr(store);
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
    let lval = self.lval.generate(config)?;
    let exp = self.exp.generate(config)?;

    if let Value::Value(irvalue) = &lval {
      let store = config.new_value_builder().store(exp, *irvalue);
      config.insert_instr(store);
    }
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
    let cond = self.cond.generate(config)?;
    let then_block = config.new_bb("%then".into());
    let end_if = config.new_bb("%endif".into());

    if let Some(else_stmt) = &self.else_stmt {
      let else_block = config.new_bb("%else".into());
      let branch = config.new_value_builder().branch(cond, then_block, else_block);
      config.insert_instr(branch);

      config.set_bb(else_block);
      else_stmt.generate(config)?;
      let jump = config.new_value_builder().jump(end_if);
      config.insert_instr(jump);
    } else {
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
    let bb_entry = config.new_bb("%entry".into());
    let bb_body = config.new_bb("%body".into());
    let bb_end = config.new_bb("%end".into());
    let jump = config.new_value_builder().jump(bb_entry);
    config.insert_instr(jump);

    config.set_bb(bb_entry);
    let cond = self.cond.generate(config)?;
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
  type Out = IrValue;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    // return constant immediately
    if let Some(v) = self.eval(config) {
      return Ok(config.new_value_builder().integer(v));
    }

    match self {
      Self::Number(v) => Ok(config.new_value_builder().integer(*v)),
      Self::LVal(lval) => {
        let value = lval.generate(config)?;
        match &value {
          Value::Const(v) => Ok(config.new_value_builder().integer(*v)),
          Value::Value(v) => {
            let load = config.new_value_builder().load(*v);
            config.insert_instr(load);
            Ok(load)
          },
        }
      },
      Self::UnaryExp(uop, exp) => {
        let value = exp.generate(config)?;
        let zero = config.new_value_builder().integer(0);
        let instr = match uop {
          UnaryOp::Neg => config.new_value_builder().binary(IrBinaryOp::Sub, zero, value),
          UnaryOp::Not => config.new_value_builder().binary(IrBinaryOp::Eq, zero, value),
        };
        config.insert_instr(instr);
        Ok(instr)
      },
      // FIXME: short circuit logic operator
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
            let lval = lhs.generate(config)?;
            let reval = config.new_bb("%reval".into());
            let short_path = config.new_bb("%short_path".into());
            let branch = config.new_value_builder().branch(lval, reval, short_path);
            config.insert_instr(branch);

            config.set_bb(reval);
            let rval = rhs.generate(config)?;
            let store = config.new_value_builder().store(rval, result);
            config.insert_instr(store);

            let jump = config.new_value_builder().jump(short_path);
            config.insert_instr(jump);

            config.set_bb(short_path);
            let load = config.new_value_builder().load(result);
            config.insert_instr(load);
            Ok(load)
          },
          BinaryOp::Or => {
            let result = config.new_value_builder().alloc(Type::get_i32());
            config.insert_instr(result);
            {
              let one = config.new_value_builder().integer(1);
              let store = config.new_value_builder().store(one, result);
              config.insert_instr(store);
            }
            let lval = lhs.generate(config)?;
            let reval = config.new_bb("%reval".into());
            let short_path = config.new_bb("%short_path".into());
            let branch = config.new_value_builder().branch(lval, short_path, reval);
            config.insert_instr(branch);

            config.set_bb(reval);
            let rval = rhs.generate(config)?;
            let store = config.new_value_builder().store(rval, result);
            config.insert_instr(store);

            let jump = config.new_value_builder().jump(short_path);
            config.insert_instr(jump);

            config.set_bb(short_path);
            let load = config.new_value_builder().load(result);
            config.insert_instr(load);
            Ok(load)
          },
          other => {
            let lval = lhs.generate(config)?;
            let rval = rhs.generate(config)?;
            let bop = other.generate(config)?;
            let binary = config.new_value_builder().binary(bop, lval, rval);
            config.insert_instr(binary);
            Ok(binary)
          }
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

// FIXME: treat ptr value in ast
impl<'ast> ProgramGen<'ast> for LVal {
  type Out = Value;
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    if let Some(v) = config.get_value(&self.ident) {
      Ok(v)
    } else {
      Err(FrontendError::UndeclaredVar(format!("{}", &self.ident)))
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