mod backend;
mod frontend;

use frontend::FrontendError;

use std::io;
use std::fmt;
use std::env::args;
use std::process::exit;
use std::fs::{read_to_string};
use koopa::back::KoopaGenerator;

type Result<T> = std::result::Result<T, Error>;

fn main() {
  if let Err(e) = compile() {
    eprintln!("{}", e);
    exit(-1);
  }
}

fn compile() -> Result<()> {
  let (mode, input, output) = parse()?;

  // read input and generate ir
  let input = read_to_string(input).map_err(Error::FileError)?;
  let ir = frontend::generate_ir(input).map_err(|e| Error::FrontendError(e))?;

  match mode {
    Mode::Koopa => KoopaGenerator::from_path(output)
      .map_err(Error::FileError)?
      .generate_on(&ir)
      .map_err(Error::IOError)?,
    Mode::Riscv => backend::generate_asm(&ir, &output)
      .map_err(Error::FileError)?,
    Mode::Perf => { unimplemented!() },
  }
  Ok(())
}

/* parse command line args */
fn parse() -> Result<(Mode, String, String)> {
  let mut args = args();
  args.next();
  if let (Some(mode), Some(input), Some(_o), Some(output)) = 
    (args.next(), args.next(), args.next(), args.next()) {
    let mode = match mode.as_str() {
      "-koopa" => Mode::Koopa,
      "-riscv" => Mode::Riscv,
      "-perf" => Mode::Perf,
      _ => return Err(Error::InvalidArgs),
    };
    Ok((mode, input, output))
  } else {
    Err(Error::InvalidArgs)
  }
}

enum Mode {
  Koopa,
  Riscv,
  Perf,
}

enum Error {
  InvalidArgs,
  FrontendError(FrontendError),
  FileError(io::Error),
  IOError(io::Error),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidArgs => write!(f, "[Invalid Args]"),
      Self::FrontendError(e) => write!(f, "[Frontend Error]: {}", e),
      Self::FileError(e) => write!(f, "[File Error]: {}", e),
      Self::IOError(e) => write!(f, "[Io Error]: {}", e),
    }
  }
}

impl fmt::Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self)
  }
}