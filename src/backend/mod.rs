/*
  backend of the compiler:
  - gen: generation for risc-v asm
*/

mod gen;

use gen::AsmGen;
use std::fs::File;
use koopa::ir::Program;

pub fn generate_asm(program: &Program, path: &str) -> Result<(), std::io::Error> {
  program.generate(&mut File::create(path)?)
}