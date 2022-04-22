// use super::gen::{ProgramGen};
use super::config::{Value, Config};
use crate::frontend::ast::*;

impl Exp {
  pub fn eval<'ast>(&'ast self, config: &mut Config<'ast>) -> Option<i32> {
    match self {
      Self::Number(v) => Some(*v),
      Self::LVal(lval) => {
        let v = config.get_value(&lval.ident).ok();
        if let Some(x) = &v {
          match x {
            Value::Const(c) => Some(*c),
            _ => None,
          }
        } else {
          None
        }
      }
      Self::UnaryExp(uop, exp) => {
        match uop {
          UnaryOp::Neg => Some(-exp.eval(config)?),
          UnaryOp::Not => Some((exp.eval(config)? == 0).into()),
        }
      },
      Self::BinaryExp(lhs, op, rhs) => {
        match op {
          BinaryOp::Mul => Some(lhs.eval(config)? * rhs.eval(config)?),
          BinaryOp::Div => (rhs.eval(config)).and_then(|rv| Some(lhs.eval(config)? / rv)),
          BinaryOp::Mod => (rhs.eval(config)).and_then(|rv| Some(lhs.eval(config)? % rv)),
          BinaryOp::Add => Some(lhs.eval(config)? + rhs.eval(config)?),
          BinaryOp::Sub => Some(lhs.eval(config)? - rhs.eval(config)?),
          BinaryOp::Lt => Some((lhs.eval(config)? < rhs.eval(config)?).into()),
          BinaryOp::Gt => Some((lhs.eval(config)? > rhs.eval(config)?).into()),
          BinaryOp::Le => Some((lhs.eval(config)? <= rhs.eval(config)?).into()),
          BinaryOp::Ge => Some((lhs.eval(config)? >= rhs.eval(config)?).into()),
          BinaryOp::Eq => Some((lhs.eval(config)? == rhs.eval(config)?).into()),
          BinaryOp::Neq => Some((lhs.eval(config)? != rhs.eval(config)?).into()),
          BinaryOp::And => (lhs.eval(config)).and_then(|lv| 
            if lv == 0 { Some(0) } else { Some((rhs.eval(config)? != 0).into()) }
          ),
          BinaryOp::Or => (lhs.eval(config)).and_then(|lv|
            if lv != 0 { Some(1) } else { Some((rhs.eval(config)? != 0).into()) }
          ),
        }
      },
      _ => None,
    }
  }
}