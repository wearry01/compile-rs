/*
  frontend of the compiler:
  - ast: definition of ast nodes
  - gen: generation for koopa ir
  - expr: evaluation of const expression
  - config: maintainance of context information
    - program
    - function
    - symbol table
      - local var table
      - global func table 
*/

mod ast;
mod gen;
mod expr;
mod config;

use gen::ProgramGen;
use config::Config;

use std::fmt;
use koopa::ir::Program;

use lalrpop_util::lalrpop_mod;

lalrpop_mod! {
  #[allow(clippy::all)]
  sysy
}

pub fn generate_ir(input: String) -> Result<Program, FrontendError> {
  let ast = sysy::CompUnitParser::new().parse(&input).map_err(
    |e| FrontendError::ParseFailure(e.to_string())
  )?;
  let mut program = Program::new();
  ast.generate(&mut Config::new(&mut program))?;
  Ok(program)
}

// Deal errors that may occur in frontend

pub enum FrontendError {
  ParseFailure(String), // Error message reported by parser
  UndeclaredVar(String),
  EvalConstExpFail,
  MultiDef(String),
}

impl fmt::Display for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::ParseFailure(err) => write!(f, "parse failure: {}", &err),
      Self::UndeclaredVar(ident) => write!(f, "identification `{}` is undeclared", &ident),
      Self::EvalConstExpFail => write!(f, "failed in eval const expr"),
      Self::MultiDef(ident) => write!(f, "identification `{}` defined multiple times", &ident),
    }
  }
}

impl fmt::Debug for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self)
  }
}