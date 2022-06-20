use std::fs::File;
use std::io::Result;
use super::format::Format;

pub enum Value {
  Null,
  Const(i32),
  Arg(usize),
  Global(String),
  Local((usize, bool)),
}

impl Value {
  pub fn is_ptr(&self) -> bool {
    if let Self::Local((_, p)) = &self {
      *p
    } else {
      false
    }
  }

  pub fn to(&self, file: &mut File, dst: &str, offset: usize) -> Result<()> {
    let mut format = Format::new(file);
    match self {
      Self::Const(i) => format.li(dst, *i),
      Self::Local((o, _)) => format.lw(dst, "sp", *o as i32),
      Self::Global(sym) => {
        format.la(dst, &sym)?;
        format.lw(dst, dst, 0)
      },
      Self::Arg(i) => {
        if *i < 8 {
          format.mv(dst, &format!("a{}", *i))?;
        } else {
          format.lw(dst, "sp", (offset + 4 * (*i - 8)) as i32)?;
        }
        Ok(())
      },
      _ => unreachable!(),
    }
  }

  pub fn addr_to(&self, file: &mut File, dst: &str) -> Result<()> {
    let mut format = Format::new(file);
    match self {
      Self::Local((o, _)) => format.addi(dst, "sp", *o as i32),
      Self::Global(sym) => format.la(dst, &sym),
      _ => unreachable!(),
    }
  }

  pub fn load(&self, file: &mut File, src: &str) -> Result<()> {
    let mut format = Format::new(file);
    match self {
      Self::Null => Ok(()),
      Self::Arg(i) => {
        if *i < 8 {
          format.mv(&format!("a{}", *i), src)?;
        } else {
          format.sw(src, "sp", (4 * (*i - 8)) as i32)?;
        }
        Ok(())
      }
      Self::Local((o, _)) => format.sw(src, "sp", *o as i32),
      Self::Global(sym) => {
        format.la("t5", &sym)?;
        format.sw(src, "t5", 0)
      },
      _ => unreachable!()
    }
  }
}

impl From<Option<&(usize, bool)>> for Value {
  fn from(x: Option<&(usize, bool)>) -> Self {
    match x {
      Some((o, i)) => Self::Local((*o, *i)),
      _ => Self::Null,
    }
  }
}