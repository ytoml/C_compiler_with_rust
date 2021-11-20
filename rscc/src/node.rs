use std::cell::RefCell;
use std::fmt::{Formatter, Display, Result};
use std::rc::Rc;

use crate::{
	token::{error_tok, TokenRef},
	typecell::TypeCell,
};

pub type NodeRef = Rc<RefCell<Node>>;

#[derive(Clone, Copy, Debug, PartialEq)]
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
	LThanNd,	// '<' // '>' や ">=" はパース時に構文木の左右を入れ替えることで調整
	LEqNd,		// "<="
	IfNd,		// "if"
	ForNd,		// "for"
	WhileNd,	// "while"
	ReturnNd,	// "return"
	BlockNd,	// {}
	CommaNd,	// ','
	FunCallNd,	// func()
	GlobalNd,	// グローバル変数(関数含む)
}

#[derive(Clone, Debug)]
pub struct Node {
	pub kind: Nodekind, // Nodeの種類
	pub token: Option<TokenRef>, // 対応する Token (エラーメッセージに必要)
	pub typ: Option<TypeCell>, 

	// プロパティとなる数値
	pub val: Option<i32>,
	pub offset: Option<usize>,// ベースポインタからのオフセット(ローカル変数時のみ)

	// 通常ノード(計算式評価)用の左右ノード
	pub left: Option<NodeRef>,
	pub right: Option<NodeRef>,

	// for (init; enter; routine) branch, if (enter) branch else els, while(enter) branch 
	pub init: Option<NodeRef>,
	pub enter: Option<NodeRef>, 
	pub routine: Option<NodeRef>, 
	pub branch: Option<NodeRef>,
	pub els: Option<NodeRef>,

	// {children}: ほんとはOptionのVecである必要はない気がするが、ジェネレータとの互換を考えてOptionに揃える
	pub children: Vec<Option<NodeRef>>,


	// グローバル変数等で使用
	pub name: Option<String>,

	// 関数に使用
	pub func_typ: Option<TypeCell>,
	pub args: Vec<Option<NodeRef>>,
	pub stmts: Option<Vec<NodeRef>>,
	pub max_offset: Option<usize>,

	// 変数時に使用
	pub is_local: bool,
}

// 並列で処理することがないものとして、グローバル変数の都合で Send/Sync を使う
unsafe impl Send for Node {}
unsafe impl Sync for Node {}

// 初期化を簡単にするためにデフォルトを定義
impl Default for Node {
	fn default() -> Node {
		Node {kind: Nodekind::DefaultNd, token: None, typ: None, val: None, offset: None, left: None, right: None, init: None, enter: None, routine: None, branch: None, els: None, children: vec![], name: None, func_typ: None, args: vec![], stmts: None, max_offset: None, is_local: false }
	}
}

static REP_NODE:usize = 40;
impl Display for Node {
	fn fmt(&self, f:&mut Formatter) -> Result {

		let mut s = format!("{}\n", "-".to_string().repeat(REP_NODE));
		let scope_attr = 
		if self.kind == Nodekind::LvarNd {
			if self.is_local { "<Local>" } else { "<Global>"}
		} else {
			""
		};
		s = format!("{}Nodekind : {:?}{}\n", s, self.kind, scope_attr);

		if let Some(e) = self.typ.as_ref() {s = format!("{}type: {}\n", s, e);}
		if let Some(e) = self.token.as_ref() {
			let tok = (*e).borrow();
			if let Some(body) = tok.body.clone() {
				s = format!("{}token: \"{}\" [{}, {}]\n", s, body, tok.line_num, tok.line_offset);
			} else {
				s = format!("{}token: [{}, {}]\n", s, tok.line_num, tok.line_offset);
			}
		}
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

		if let Some(e) = self.func_typ.as_ref() {s = format!("{}function type: {}\n", s, e);}
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

// $tok は &Token を渡す
#[macro_export]
macro_rules! error_with_node {
	($fmt: expr, $tok: expr) => (
		use crate::node::error_nod;
		error_nod($fmt, $tok);
	);

	($fmt: expr, $tok: expr, $($arg: tt)*) => (
		use crate::node::error_nod;
		error_nod(format!($fmt, $($arg)*).as_str(), $tok);
	);
}

pub fn error_nod(msg: &str, node: &Node) -> ! {
	// token.line_offset は token.len 以上であるはずなので負になる可能性をチェックしない
	error_tok(msg, &*node.token.as_ref().unwrap().borrow());
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
		println!("{}", Node::default());
		let node: Node = Node {
			kind: Nodekind::GlobalNd,
			stmts: Some(vec![
				Rc::new(RefCell::new(Node::default())),
				Rc::new(RefCell::new(Node {kind: Nodekind::AddNd, ..Default::default()})),
			]),
			..Default::default()
		};
		println!("{}", node);
	}
}