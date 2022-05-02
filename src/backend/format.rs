use std::fs::File;
use std::io::{Write, Result};

pub struct Format<'io> {
  file: &'io mut File,
}

impl<'io> Format<'io> {
  pub fn new(file: &'io mut File) -> Self {
    Self { file }
  }

  pub fn li(&mut self, dst: &str, imm: i32) -> Result<()> {
    writeln!(self.file, "\tli {dst}, {imm}")
  }

  pub fn la(&mut self, dst: &str, sym: &str) -> Result<()> {
    writeln!(self.file, "\tla {dst}, {sym}")
  }

  pub fn mv(&mut self, dst: &str, src: &str) -> Result<()> {
    if dst != src {
      writeln!(self.file, "\tmv {dst}, {src}")?;
    }
    Ok(())
  }

  pub fn bop(&mut self, op: &str, dst: &str, lv: &str, rv: &str) -> Result<()> {
    writeln!(self.file, "\t{op} {dst}, {lv}, {rv}")
  }

  pub fn uop(&mut self, op: &str, dst: &str, src: &str) -> Result<()> {
    writeln!(self.file, "\t{op} {dst}, {src}")
  }

  pub fn addi(&mut self, dst: &str, src: &str, imm: i32) -> Result<()> {
    if imm >= -2048 && imm <= 2047 {
      writeln!(self.file, "\taddi {dst}, {src}, {imm}")
    } else {
      self.li("t6", imm)?;
      writeln!(self.file, "\tadd {dst}, {src}, t6")
    }
  }

  pub fn muli(&mut self, dst: &str, src: &str, imm: i32) -> Result<()> {
    self.li("t6", imm)?;
    writeln!(self.file, "\tmul {dst}, {src}, t6")
  }

  pub fn sw(&mut self, src: &str, base: &str, offset: i32) -> Result<()> {
    self.addi("t6", base, offset)?;
    writeln!(self.file, "\tsw {src}, 0(t6)")
  }

  pub fn lw(&mut self, dst: &str, base: &str, offset: i32) -> Result<()> {
    self.addi("t6", base, offset)?;
    writeln!(self.file, "\tlw {dst}, 0(t6)")
  }

  pub fn bnez(&mut self, cond: &str, label: &str) -> Result<()> {
    writeln!(self.file, "\tbnez {cond}, {label}")
  }

  pub fn j(&mut self, label: &str) -> Result<()> {
    writeln!(self.file, "\tj {label}")
  }
  
  pub fn call(&mut self, func: &str) -> Result<()> {
    writeln!(self.file, "\tcall {func}")
  }
}