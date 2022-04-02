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
  pub exp: Exp,
}

pub enum Exp {
  Number(i32),
  Exp(Box<Exp>),
  UnaryExp(UnaryOp, Box<Exp>),
  BinaryExp(Box<Exp>, BinaryOp, Box<Exp>),
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

/*
CompUnit    ::= FuncDef;

FuncDef     ::= FuncType IDENT "(" ")" Block;
FuncType    ::= "int";

Block       ::= "{" Stmt "}";
Stmt        ::= "return" Exp ";";

Exp         ::= LOrExp;
PrimaryExp  ::= "(" Exp ")" | Number;
Number      ::= INT_CONST;
UnaryExp    ::= PrimaryExp | UnaryOp UnaryExp;
UnaryOp     ::= "+" | "-" | "!";
MulExp      ::= UnaryExp | MulExp ("*" | "/" | "%") UnaryExp;
AddExp      ::= MulExp | AddExp ("+" | "-") MulExp;
RelExp      ::= AddExp | RelExp ("<" | ">" | "<=" | ">=") AddExp;
EqExp       ::= RelExp | EqExp ("==" | "!=") RelExp;
LAndExp     ::= EqExp | LAndExp "&&" EqExp;
LOrExp      ::= LAndExp | LOrExp "||" LAndExp;
*/