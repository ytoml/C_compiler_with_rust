// 再帰下降構文のパーサ
use crate::tokenizer::{Token, Tokenkind, consume, consume_kind, expect, expect_number, expect_ident, is_ident, at_eof};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::fmt::{Formatter, Display, Result};

static LOCALS: Lazy<Mutex<HashMap<String, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static LVAR_MAX_OFFSET: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

#[derive(Debug, PartialEq)]
pub enum Nodekind {
	DefaultNd, // defalut
	AddNd, // '+'
	SubNd, // '-'
	MulNd, // '*'
	DivNd, // '/'
	AssignNd, // '='
	LvarNd, // 左辺値
	NumNd, // 数値
	EqNd, // "=="
	NEqNd, // "!="
	GThanNd, // '>'
	GEqNd, // ">="
	LThanNd, // '<'
	LEqNd, // "<="
	IfNd, // if
	ElseNd, // else
	ForNd, // for
	WhileNd, // while
	ReturnNd, // return
}

pub struct Node {
	pub kind: Nodekind,
	// 通常ノード(計算式評価)用の左右ノード
	pub left: Option<Rc<RefCell<Node>>>,
	pub right: Option<Rc<RefCell<Node>>>,

	// for (init; enter; routine) branch, if (enter) branch, while(enter) branch 
	pub init: Option<Rc<RefCell<Node>>>,
	pub enter: Option<Rc<RefCell<Node>>>, 
	pub routine: Option<Rc<RefCell<Node>>>, 

	// プロパティとなる数値
	pub val: Option<i32>,
	pub offset: Option<usize>,// ベースポインタからのオフセット(ローカル変数時のみ)
}

// 初期化を簡単にするためにデフォルトを定義
impl Default for Node {
	fn default() -> Node {
		Node { kind: Nodekind::DefaultNd, left: None, right: None, init: None, enter: None, routine: None, val: None, offset: None,}
	}
}

static REP_NODE:usize = 40;
impl Display for Node {
	fn fmt(&self, f:&mut Formatter) -> Result {

		let mut s = format!("{}\n", "-".to_string().repeat(REP_NODE));
		s = format!("{}Nodekind : {:?}\n", s, self.kind);

		if let Some(e) = self.left.as_ref() {
			s = format!("{}left: exist(kind:{:?})\n", s, e.borrow().kind);
		} else {
			s = format!("{}left: not exist\n", s);
		}

		if let Some(e) = self.right.as_ref() {
			s = format!("{}right: exist(kind:{:?})\n", s, e.borrow().kind);
		} else {
			s = format!("{}right: not exist\n", s);
		}

		if let Some(e) = self.val.as_ref() {
			s = format!("{}val: {}\n", s, e);
		} else {
			s = format!("{}val: not exist\n", s);
		}

		write!(f, "{}", s)
	}
}


// ノードの作成
fn new_node(kind: Nodekind, left: Rc<RefCell<Node>>, right: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
	Rc::new(RefCell::new(
		Node {
			kind: kind,
			left: Some(left), 
			right: Some(right),
			.. Default::default()
		}
	))
}

// 数字に対応するノード
fn new_node_num(val: i32) -> Rc<RefCell<Node>> {
	Rc::new(RefCell::new(
		Node {
			kind: Nodekind::NumNd,
			val: Some(val),
			.. Default::default()
		}
	))
}

// 左辺値(今のうちはローカル変数)に対応するノード(現在は1文字のみ)
fn new_node_lvar(name: impl Into<String>) -> Rc<RefCell<Node>> {
	let name: String = name.into();
	let offset;


	// デッドロック回避のため、フラグを用意してmatch内で再度LOCALS(<変数名, オフセット>のHashMap)にアクセスしないようにする
	let mut not_found: bool = false;
	match LOCALS.lock().unwrap().get(&name) {
		Some(_offset) => {
			offset = *_offset;
		}, 
		// 見つからなければオフセットの最大値を伸ばす
		None => {
			*LVAR_MAX_OFFSET.lock().unwrap() += 8; 
			offset = *LVAR_MAX_OFFSET.lock().unwrap();
			not_found = true;
		}
	}

	if not_found {
		LOCALS.lock().unwrap().insert(name, offset); 
	}
	
	Rc::new(RefCell::new(
		Node {
			kind: Nodekind::LvarNd,
			offset: Some(offset),
			.. Default::default()
		}

	))
}


// 生成規則: program = stmt*
pub fn program(token_ptr: &mut Rc<RefCell<Token>>) -> Vec<Rc<RefCell<Node>>> {
	let mut statements :Vec<Rc<RefCell<Node>>> = Vec::new();
	while !at_eof(token_ptr) {
		statements.push(stmt(token_ptr));
	}

	statements
}


// 生成規則: stmt = 
// expr? ";"
// "if" "(" expr ")" stmt
// "while" "(" expr ")" stmt
// "for" "(" expr? ";" expr? ";" expr? ")" stmt 
// "return" expr ";"
// まだブロックには対応していない(一気に実装してごちゃつくのを防ぐため
fn stmt(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr: Rc<RefCell<Node>>;

	// if consume(token_ptr, ";") {return node_ptr;}

	if consume(token_ptr, "if") {
		// PENDING: 木構造考えてnode_ptrをどう更新するか決める必要がある
		expect(token_ptr, "(");
		node_ptr = expr(token_ptr);

		expect(token_ptr, ")");
		node_ptr = stmt(token_ptr);

		if consume(token_ptr, "else") {
			node_ptr = stmt(token_ptr);
		}
		

	} else if consume(token_ptr, "while") {
		// PENDING: 同上
		expect(token_ptr, "(");
		node_ptr = expr(token_ptr);
		expect(token_ptr, ")");

		node_ptr = stmt(token_ptr);

	} else if consume(token_ptr, "for") {
		// PENDING: 同上
		expect(token_ptr, "(");

		if consume(token_ptr, ";") {
			node_ptr = expr(token_ptr);
		} else {expect(token_ptr, ";")}


		if consume(token_ptr, ";") {
			node_ptr = expr(token_ptr);
		} else {expect(token_ptr, ";")}


		if consume(token_ptr, ")") {
			node_ptr = expr(token_ptr);
		} else {expect(token_ptr, ")")}
		
		node_ptr = stmt(token_ptr);

	} else if consume_kind(token_ptr, Tokenkind::ReturnTk) {
		// ReturnNdはここでしか生成しないため、ここにハードコードする
		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::ReturnNd,
				left: Some(expr(token_ptr)),
				..Default::default()
			}
		));
	} else {
		node_ptr = expr(token_ptr);
	}

	expect(token_ptr, ";");

	node_ptr
}

// 生成規則: expr = assign
pub fn expr(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	assign(token_ptr)
}

// 生成規則: assign = equality ("=" assign)?
fn assign(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = equality(token_ptr);
	if consume(token_ptr, "=") {
		node_ptr = new_node(Nodekind::AssignNd, node_ptr,  assign(token_ptr));
	}
	
	node_ptr
}

// 生成規則: equality = relational ("==" relational | "!=" relational)?
pub fn equality(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = relational(token_ptr);
	if consume(token_ptr, "==") {
		node_ptr = new_node(Nodekind::EqNd, node_ptr, relational(token_ptr));

	} else if consume(token_ptr, "!=") {
		node_ptr = new_node(Nodekind::NEqNd, node_ptr, relational(token_ptr));
	}

	node_ptr
}

// 生成規則: relational = add ("<" add | "<=" add | ">" add | ">=" add)*
fn relational(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = add(token_ptr);

	loop {
		if consume(token_ptr, "<") {
			node_ptr = new_node(Nodekind::LThanNd, node_ptr, add(token_ptr));

		} else if consume(token_ptr, "<=") {
			node_ptr = new_node(Nodekind::LEqNd, node_ptr, add(token_ptr));

		} else if consume(token_ptr, ">") {
			node_ptr = new_node(Nodekind::GThanNd, node_ptr, add(token_ptr));

		} else if consume(token_ptr, ">=") {
			node_ptr = new_node(Nodekind::GEqNd, node_ptr, add(token_ptr));

		} else{
			break;
		}
	}


	node_ptr

}

// 生成規則: add = mul ("+" mul | "-" mul)*
pub fn add(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = mul(token_ptr);

	loop {
		if consume(token_ptr, "+") {
			node_ptr = new_node(Nodekind::AddNd, node_ptr, mul(token_ptr));

		} else if consume(token_ptr, "-") {
			node_ptr = new_node(Nodekind::SubNd, node_ptr, mul(token_ptr));

		} else {
			break;
		}
	}

	node_ptr
}

// 生成規則: mul = unary ("*" unary | "/" unary)*
fn mul(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = unary(token_ptr);

	loop {
		if consume(token_ptr, "*") {
			node_ptr = new_node(Nodekind::MulNd, node_ptr, unary(token_ptr));

		} else if consume(token_ptr, "/") {
			node_ptr = new_node(Nodekind::DivNd, node_ptr, unary(token_ptr));

		} else {
			break;
		}
	}

	node_ptr
}


// 生成規則: unary = ("+" | "-")? primary
fn unary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr;

	if consume(token_ptr, "+") {
		node_ptr = primary(token_ptr);

	} else if consume(token_ptr, "-") {
		// 単項演算のマイナスは0から引く形にする。
		node_ptr = new_node(Nodekind::SubNd, new_node_num(0), primary(token_ptr));

	} else {
		node_ptr = primary(token_ptr);
	}

	node_ptr
}



// 生成規則: primary = num | ident | "(" expr ")"
fn primary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr;

	if consume(token_ptr, "(") {
		node_ptr = expr(token_ptr);

		expect(token_ptr, ")");

	} else if is_ident(token_ptr) {
		node_ptr = new_node_lvar(expect_ident(token_ptr));

	} else {
		node_ptr = new_node_num(expect_number(token_ptr));

	}

	node_ptr
}

mod tests {
	use super::*;

	#[test]
	fn test_display() {
		println!("test_display{}", "-".to_string().repeat(40));
		let node = new_node_num(0);
		println!("{}", (*node).borrow());
	}
}