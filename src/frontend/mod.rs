/*
  frontend of the compiler:
  - ast: definition of ast nodes
  - gen: generation for koopa ir
*/

mod ast;
mod gen;

use gen::ProgramGen;

use std::fmt;
use koopa::ir::Program;

use lalrpop_util::lalrpop_mod;

lalrpop_mod! {
  #[allow(clippy::all)]
  sysy
}

pub fn generate_ir(input: String) -> Result<Program, FrontendError> {
  let ast = sysy::CompUnitParser::new()
  .parse(&input)
  .map_err(|e| FrontendError::ParseFailure(e.to_string()))?;
  let mut program = Program::new();
  ast.generate(&mut program)?;
  Ok(program)
}

// Deal errors that may occur in frontend

pub enum FrontendError {
  ParseFailure(String), // Error message reported by other crates
}

impl fmt::Display for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::ParseFailure(err) => write!(f, "[parse failure]: {}", &err),
    }
  }
}

impl fmt::Debug for FrontendError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self)
  }
}