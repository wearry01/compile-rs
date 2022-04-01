pub struct CompUnit {
  pub func_def: FuncDef,
}

pub struct FuncDef {
  pub func_type: FuncType,
  pub ident: String,
  pub block: Block,
}

pub enum FuncType {
  Int,
}

pub struct Block {
  pub stmt: Stmt,
}

pub struct Stmt {
  pub ret_value: i32,
}