use std::cell::RefCell;
use std::fmt::{Formatter, Display, Result};
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum Nodekind {
	DefaultNd,	// defalut
	AddNd,		// '+'
	SubNd,		// '-'
	MulNd,		// '*'
	DivNd,		// '/'
	ModNd,		// '%'
	LShiftNd,	// "<<"
	RShiftNd,	// ">>"
	BitAndNd,	// '&'
	BitOrNd,	// '|'
	BitXorNd,	// '^'
	BitNotNd,	// '~'
	LogAndNd,	// "&&"
	LogOrNd,	// "||"
	LogNotNd,	// '!'
	AssignNd,	// '='
	LvarNd,		// 左辺値
	NumNd,		// 数値
	AddrNd,		// アドレス参照(&)
	DerefNd,	// アドレスの値を読む(*)
	EqNd,		// "=="
	NEqNd,		// "!="
	GThanNd,	// '>'
	GEqNd,		// ">="
	LThanNd,	// '<'
	LEqNd,		// "<="
	IfNd,		// "if"
	ForNd,		// "for"
	WhileNd,	// "while"
	ReturnNd,	// "return"
	BlockNd,	// {}
	CommaNd,	// ','
	FuncNd,		// func()
	FuncDecNd,	// 関数の宣言
}

pub struct Node {
	pub kind: Nodekind, // Nodeの種類

	// プロパティとなる数値
	pub val: Option<i32>,
	pub offset: Option<usize>,// ベースポインタからのオフセット(ローカル変数時のみ)

	// 通常ノード(計算式評価)用の左右ノード
	pub left: Option<Rc<RefCell<Node>>>,
	pub right: Option<Rc<RefCell<Node>>>,

	// for (init; enter; routine) branch, if (enter) branch else els, while(enter) branch 
	pub init: Option<Rc<RefCell<Node>>>,
	pub enter: Option<Rc<RefCell<Node>>>, 
	pub routine: Option<Rc<RefCell<Node>>>, 
	pub branch: Option<Rc<RefCell<Node>>>,
	pub els: Option<Rc<RefCell<Node>>>,

	// {children}: ほんとはOptionのVecである必要はない気がするが、ジェネレータとの互換を考えてOptionに揃える
	pub children: Vec<Option<Rc<RefCell<Node>>>>,

	// func の引数を保存する: 
	pub args: Vec<Option<Rc<RefCell<Node>>>>,
	// func 時に使用(もしかしたらグローバル変数とかでも使うかも？)
	pub name: Option<String>,
	// 関数宣言時に使用
	pub stmts: Option<Vec<Rc<RefCell<Node>>>>, // プログラム情報
	pub max_offset: Option<usize>

}

// 初期化を簡単にするためにデフォルトを定義
impl Default for Node {
	fn default() -> Node {
		Node { kind: Nodekind::DefaultNd, val: None, offset: None, left: None, right: None, init: None, enter: None, routine: None, branch: None, els: None, children: vec![], args: vec![], name: None, stmts: None, max_offset: None}
	}
}

static REP_NODE:usize = 40;
impl Display for Node {
	fn fmt(&self, f:&mut Formatter) -> Result {

		let mut s = format!("{}\n", "-".to_string().repeat(REP_NODE));
		s = format!("{}Nodekind : {:?}\n", s, self.kind);

		if let Some(e) = self.val.as_ref() {s = format!("{}val: {}\n", s, e);}
		if let Some(e) = self.name.as_ref() {s = format!("{}name: {}\n", s, e);}
		if let Some(e) = self.offset.as_ref() {s = format!("{}offset: {}\n", s, e);} 
		if let Some(e) = self.left.as_ref() {s = format!("{}left: exist(kind:{:?})\n", s, e.borrow().kind);} 
		if let Some(e) = self.right.as_ref() {s = format!("{}right: exist(kind:{:?})\n", s, e.borrow().kind);}
		if let Some(e) = self.init.as_ref() {s = format!("{}init: exist(kind:{:?})\n", s, e.borrow().kind);}
		if let Some(e) = self.enter.as_ref() {s = format!("{}enter: exist(kind:{:?})\n", s, e.borrow().kind);}
		if let Some(e) = self.routine.as_ref() {s = format!("{}routine: exist(kind:{:?})\n", s, e.borrow().kind);}
		if let Some(e) = self.branch.as_ref() {s = format!("{}branch: exist(kind:{:?})\n", s, e.borrow().kind);}
		if let Some(e) = self.els.as_ref() {s = format!("{}els: exist(kind:{:?})\n", s, e.borrow().kind);}

		if self.children.len() > 0 {
			s = format!("{}children: exist\n", s);
			for node in &self.children {
				if let Some(e) = node.as_ref() {s = format!("{}->kind:{:?}\n", s, e.borrow().kind);}
				else {s = format!("{}->NULL\n", s);}
			}
		}

		if self.args.len() > 0 {
			s = format!("{}args: exist\n", s);
			for node in &self.args {
				if let Some(e) = node.as_ref() {s = format!("{}->kind:{:?}\n", s, e.borrow().kind);}
				else {s = format!("{}->NULL\n", s);}
			}
		}

		if let Some(e) = self.stmts.as_ref() {s = format!("{}stmts: exist({})\n", s, e.len());}
		if let Some(e) = self.max_offset.as_ref() {s = format!("{}max_offset: {}\n", s, e);}

		write!(f, "{}", s)
	}
}