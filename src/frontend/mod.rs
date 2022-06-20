/*
  frontend of the compiler:
  - ast: definition of ast nodes
  - gen: generation for koopa ir
  - expr: evaluation of const expression
  - config: maintainance of context information
    - program
    - function
    - symbol table
      - current (local) var table
      - global func table 
    - while loop entries and exits
  - value: 
    - deal different value types
      - not a value (void)
      - integer value (int a)
      - ptr value (int *a)
      - array ptr (int *a[])
    - implementation of initializers
*/

mod ast;
mod gen;
mod expr;
mod value;
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
  UndeclaredId(String),
  EvalConstExpFail,
  MultiDef(String),
  InvalidInitializer,
  InvalidValueType,
}

impl fmt::Display for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::ParseFailure(err) => write!(f, "parse failure: {}", &err),
      Self::UndeclaredId(ident) => write!(f, "ident `{}` is undeclared", &ident),
      Self::EvalConstExpFail => write!(f, "failed in eval const expr"),
      Self::MultiDef(ident) => write!(f, "ident `{}` defined multiple times", &ident),
      Self::InvalidInitializer => write!(f, "invalid initializer detected"),
      Self::InvalidValueType => write!(f, "invalid value type detected"),
    }
  }
}

impl fmt::Debug for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self)
  }
}