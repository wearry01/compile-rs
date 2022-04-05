pub struct CompUnit {
  pub func_def: FuncDef,
}

pub enum Decl {
  ConstDecl(ConstDecl),
  VarDecl(VarDecl),
}

pub struct ConstDecl {
  pub item: Vec<ConstDef>,
}

pub struct ConstDef {
  pub ident: String,
  pub initial: ConstInitVal,
}

pub enum ConstInitVal {
  Exp(ConstExp),
}

pub struct VarDecl {
  pub item: Vec<VarDef>,
}

pub struct VarDef {
  pub ident: String,
  pub initial: Option<InitVal>,
}

pub enum InitVal {
  Exp(Exp),
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
  pub item: Vec<BlockItem>,
}

pub enum BlockItem {
  Decl(Decl),
  Stmt(Stmt),
}

pub enum Stmt {
  Assign(Assign),
  ExpStmt(ExpStmt),
  Block(Block),
  Return(Exp),
}

pub struct ExpStmt {
  pub exp: Option<Exp>,
}

pub enum Exp {
  Number(i32),
  LVal(LVal),
  Exp(Box<Exp>),
  UnaryExp(UnaryOp, Box<Exp>),
  BinaryExp(Box<Exp>, BinaryOp, Box<Exp>),
}

pub struct ConstExp {
  pub exp: Exp,
}

pub struct Assign {
  pub lval: LVal,
  pub exp: Exp,
}

pub struct LVal {
  pub ident: String,
}

pub enum UnaryOp {
  Neg,
  Not
}

pub enum BinaryOp {
  // MulExp
  Mul,
  Div,
  Mod,
  // AddExp
  Add,
  Sub,
  // RelExp
  Lt,
  Le,
  Gt,
  Ge,
  // EqExp
  Eq,
  Neq,
  // AndExp
  And,
  // OrExp
  Or,
}