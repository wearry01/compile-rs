use crate::frontend::ast::*;

impl Exp {
  pub fn eval(&self) -> Option<i32> {
    match self {
      Self::Number(v) => Some(*v),
      Self::Exp(exp) => exp.eval(),
      Self::UnaryExp(uop, exp) => {
        match uop {
          UnaryOp::Neg => Some(-exp.eval()?),
          UnaryOp::Not => Some((exp.eval()? == 0).into()),
        }
      },
      Self::BinaryExp(lhs, op, rhs) => {
        match op {
          BinaryOp::Mul => Some(lhs.eval()? * rhs.eval()?),
          BinaryOp::Div => (rhs.eval()).and_then(|rv| Some(lhs.eval()? / rv)),
          BinaryOp::Mod => (rhs.eval()).and_then(|rv| Some(lhs.eval()? % rv)),
          BinaryOp::Add => Some(lhs.eval()? + rhs.eval()?),
          BinaryOp::Sub => Some(lhs.eval()? - rhs.eval()?),
          BinaryOp::Lt => Some((lhs.eval()? < rhs.eval()?).into()),
          BinaryOp::Gt => Some((lhs.eval()? > rhs.eval()?).into()),
          BinaryOp::Le => Some((lhs.eval()? <= rhs.eval()?).into()),
          BinaryOp::Ge => Some((lhs.eval()? >= rhs.eval()?).into()),
          BinaryOp::Eq => Some((lhs.eval()? == rhs.eval()?).into()),
          BinaryOp::Neq => Some((lhs.eval()? != rhs.eval()?).into()),
          // short circuit
          BinaryOp::And => (lhs.eval()).and_then(|lv| 
            if lv == 0 { Some(0) } else { Some((rhs.eval()? != 0).into()) }
          ),
          BinaryOp::Or => (lhs.eval()).and_then(|lv|
            if lv != 0 { Some(1) } else { Some((rhs.eval()? != 0).into()) }
          ),
        }
      }
    }
  }
}