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
      Self::Exp(exp) => exp.generate(config),
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
      Self::Return(exp) => {
        let value = exp.generate(config)?;
        let ret_val = config.ret_val().unwrap();
        let instr = config.new_value_builder().store(value, ret_val);
        config.insert_instr(instr);
      },
      Self::Assign(assign) => assign.generate(config)?,
    }
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
          Value::Value(v) => {
            let load = config.new_value_builder().load(*v);
            config.insert_instr(load);
            Ok(load)
          },
          Value::Const(v) => {
            let cst = config.new_value_builder().integer(*v);
            Ok(cst)
          }
        }
      },
      Self::Exp(exp) => exp.generate(config),
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
        let lval = lhs.generate(config)?;
        let rval = rhs.generate(config)?;
        let op = op.generate(config)?;
        let instr = config.new_value_builder().binary(op, lval, rval);
        config.insert_instr(instr);
        Ok(instr)
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

impl<'ast> ProgramGen<'ast> for Assign {
  type Out = ();
  fn generate(&'ast self, config: &mut Config<'ast>) -> Result<Self::Out> {
    let lval = self.lval.generate(config)?;
    let exp = self.exp.generate(config)?;

    if let Value::Value(irvalue) = &lval {
      let instr = config.new_value_builder().store(exp, *irvalue);
      config.insert_instr(instr);
    }

    Ok(())
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