/*
  backend of the compiler:
  - gen: generation for risc-v asm
  - config: global configuration
  - value: deal ptr/alloc
  - format: output asm properly
*/

mod gen;
mod config;
mod format;
mod value;

use gen::AsmGen;
use config::Config;
use std::fs::File;
use koopa::ir::Program;

pub fn generate_asm(program: &Program, path: &str) -> Result<(), std::io::Error> {
  program.generate(&mut File::create(path)?, &mut Config::new(program))
}