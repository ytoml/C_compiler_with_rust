use crate::exit_eprintln;
// 再帰下降構文のパーサ
use crate::tokenizer::{Token, Tokenkind, consume, consume_kind, expect, expect_number, expect_ident, is_ident, at_eof};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Formatter, Display, Result};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LOCALS: Lazy<Mutex<HashMap<String, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static LVAR_MAX_OFFSET: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

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
	NotNd,		// '!'
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
	LogAndNd,	// "&&"
	LogOrNd,	// "||"
	IfNd,		// if
	ForNd,		// for
	WhileNd,	// while
	ReturnNd,	// return
	BlockNd,	// {}
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


// ノードの作成
fn new_node_calc(kind: Nodekind, left: Rc<RefCell<Node>>, right: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
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

// 左辺値(今のうちはローカル変数)に対応するノード
fn new_node_lvar(name: impl Into<String>) -> Rc<RefCell<Node>> {
	let name: String = name.into();
	let offset;


	// デッドロック回避のため、フラグを用意してmatch内で再度LOCALS(<変数名, オフセット>のHashMap)にアクセスしないようにする
	let mut not_found: bool = false;
	match LOCALS.lock().unwrap().get(&name) {
		Some(offset_) => {
			offset = *offset_;
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

// 生成規則: program = ident "(" (expr ",")* expr? ")" "{" stmt* "}"
pub fn program(token_ptr: &mut Rc<RefCell<Token>>) -> Vec<Rc<RefCell<Node>>> {
	let mut globals : Vec<Rc<RefCell<Node>>> = Vec::new();

	while !at_eof(token_ptr) {
		// トップレベル(グローバルスコープ)では関数宣言のみができる
		
		let mut statements : Vec<Rc<RefCell<Node>>> = Vec::new();
		let func_name = expect_ident(token_ptr);
		expect(token_ptr, "(");
		// 引数を6つまでサポート
		let mut args: Vec<Option<Rc<RefCell<Node>>>> = vec![];
		if !consume(token_ptr, ")") {
			// 引数が1つ以上あるパターン
			let mut argc: usize = 0;
			loop {
				if argc >= 6 {
					exit_eprintln!("現在7つ以上の引数はサポートされていません。");
				}
				if at_eof(token_ptr) {exit_eprintln!("関数宣言の\'(\'にマッチする\')\'が見つかりません。");}
				args.push(Some(expr(token_ptr)));
				argc += 1;

				// ','が読めたなら次の引数があるが、なければ引数列挙が終わらなければならない
				if !consume(token_ptr, ",") {
					expect(token_ptr, ")");
					break;
				}
			}
		}

		let mut has_return : bool = false;
		expect(token_ptr, "{");
		while !consume(token_ptr, "}") {
			has_return |= (**token_ptr).borrow().kind == Tokenkind::ReturnTk; // return がローカルの最大のスコープに出現するかどうかを確認 (ブロックでネストされていると対応できないのが難点…)
			statements.push(stmt(token_ptr));
		}

		if !has_return {
			statements.push(
				Rc::new(RefCell::new(
					Node {
						kind: Nodekind::ReturnNd,
						left: Some(new_node_num(0)),
						..Default::default()
					}
				))
			)
		}

		let global = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::FuncDecNd,
				name: Some(func_name),
				args: args,
				stmts: Some(statements),
				max_offset: Some(*LVAR_MAX_OFFSET.lock().unwrap()),
				..Default::default()
			}
		));
		// 関数宣言が終わるごとにローカル変数の管理情報をクリア(offset や name としてノードが持っているのでこれ以上必要ない)
		LOCALS.lock().unwrap().clear();
		*LVAR_MAX_OFFSET.lock().unwrap() = 0;

		globals.push(global);
	}
	
	globals
}


// 生成規則: stmt = 
// expr? ";" | 
// "{" stmt* "}" | 
// "if" "(" expr ")" stmt ("else" stmt)? | ...(今はelse ifは実装しない)
// "while" "(" expr ")" stmt | 
// "for" "(" expr? ";" expr? ";" expr? ")" stmt |
// "return" expr? ";"
fn stmt(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr: Rc<RefCell<Node>>;

	// exprなしのパターン: 実質NumNd 0があるのと同じと捉えれば良い
	if consume(token_ptr, ";") {
		node_ptr = new_node_num(0);
		return node_ptr;
	}

	if consume(token_ptr, "{") {
		let mut children: Vec<Option<Rc<RefCell<Node>>>> = vec![];
		loop {
			if !consume(token_ptr, "}") {
				if at_eof(token_ptr) {exit_eprintln!("\'{{\'にマッチする\'}}\'が見つかりません。");}
				children.push(Some(stmt(token_ptr)));
			} else {
				break;
			}
		}

		node_ptr = Rc::new(RefCell::new(
			Node{
				kind: Nodekind::BlockNd,
				children: children,
				..Default::default()
			}
		));

	} else if consume(token_ptr, "if") {
		expect(token_ptr, "(");
		let enter= Some(expr(token_ptr));

		expect(token_ptr, ")");
		let branch = Some(stmt(token_ptr));

		if consume(token_ptr, "else") {
			let els = Some(stmt(token_ptr));
			node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::IfNd,
				enter: enter,
				branch: branch,
				els: els,
				..Default::default()
				}
			));
		} else {
			node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::IfNd,
				enter: enter,
				branch: branch,
				..Default::default()
				}
			));
		}
		

	} else if consume(token_ptr, "while") {
		expect(token_ptr, "(");
		let enter = Some(expr(token_ptr));
		expect(token_ptr, ")");

		let branch = Some(stmt(token_ptr));

		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::WhileNd,
				enter: enter,
				branch: branch,
				..Default::default()
			}
		));

	} else if consume(token_ptr, "for") {
		expect(token_ptr, "(");

		let init: Option<Rc<RefCell<Node>>>;
		let enter: Option<Rc<RefCell<Node>>>;
		let routine: Option<Rc<RefCell<Node>>>;
		let branch: Option<Rc<RefCell<Node>>>;

		if consume(token_ptr, ";") {
			// consumeできた場合exprが何も書かれていないことに注意
			init = None;
		} else {
			init = Some(expr(token_ptr));
			expect(token_ptr, ";");
		}

		// concumeの条件分岐について同上なので注意
		if consume(token_ptr, ";") {
			enter = None;
		} else {
			enter = Some(expr(token_ptr));
			expect(token_ptr, ";")
		}

		if consume(token_ptr, ")") {
			routine = None;
		} else {
			routine = Some(expr(token_ptr));
			expect(token_ptr, ")")
		}

		branch = Some(stmt(token_ptr));
		
		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::ForNd,
				init: init,
				enter: enter,
				routine: routine,
				branch: branch,
				..Default::default()
			}
		));

	} else if consume_kind(token_ptr, Tokenkind::ReturnTk) {
		let left_ptr: Rc<RefCell<Node>>;

		// exprなしのパターン: 実質NumNd 0があるのと同じと捉えれば良い
		if consume(token_ptr, ";") {
			left_ptr = new_node_num(0);
		} else {
			left_ptr = expr(token_ptr);
			expect(token_ptr, ";");
		}

		// ReturnNdはここでしか生成しないため、ここにハードコードする
		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::ReturnNd,
				left: Some(left_ptr),
				..Default::default()
			}
		));

	} else {
		node_ptr = expr(token_ptr);
		expect(token_ptr, ";");

	}

	node_ptr
}

// 生成規則: expr = assign
pub fn expr(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	assign(token_ptr)
}


// 禁止代入(例えば x + y = 10; や x & y = 10; など)は generator 側で弾く
// 生成規則: assign = logor ("=" assign)?
fn assign(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = logor(token_ptr);
	if consume(token_ptr, "=") {
		node_ptr = new_node_calc(Nodekind::AssignNd, node_ptr,  assign(token_ptr));
	}
	
	node_ptr
}

// 生成規則: logor = logand ("||" logand)*
fn logor(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = logand(token_ptr);
	while consume(token_ptr, "||") {
		node_ptr = new_node_calc(Nodekind::LogOrNd, node_ptr, logand(token_ptr));
	}

	node_ptr
}

// 生成規則: logand = bitor ("&&" bitor)*
fn logand(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = bitor(token_ptr);
	while consume(token_ptr, "&&") {
		node_ptr = new_node_calc(Nodekind::LogAndNd, node_ptr, bitor(token_ptr));
	}

	node_ptr
}

// 生成規則: bitor = bitxor ("|" bitxor)*
fn bitor(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = bitxor(token_ptr);
	while consume(token_ptr, "|") {
		node_ptr = new_node_calc(Nodekind::BitOrNd, node_ptr, bitxor(token_ptr));
	}

	node_ptr
}

// 生成規則: bitxor = bitand ("^" bitand)*
fn bitxor(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = bitand(token_ptr);
	while consume(token_ptr, "^") {
		node_ptr = new_node_calc(Nodekind::BitXorNd, node_ptr, bitand(token_ptr));
	}

	node_ptr
}

// 生成規則: bitand = equality ("&" equality)*
fn bitand(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = equality(token_ptr);
	while consume(token_ptr, "&") {
		node_ptr = new_node_calc(Nodekind::BitAndNd, node_ptr, equality(token_ptr));
	}

	node_ptr
}


// 生成規則: equality = relational ("==" relational | "!=" relational)?
pub fn equality(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = relational(token_ptr);
	if consume(token_ptr, "==") {
		node_ptr = new_node_calc(Nodekind::EqNd, node_ptr, relational(token_ptr));

	} else if consume(token_ptr, "!=") {
		node_ptr = new_node_calc(Nodekind::NEqNd, node_ptr, relational(token_ptr));
	}

	node_ptr
}

// 生成規則: relational = shift ("<" shift | "<=" shift | ">" shift | ">=" shift)*
fn relational(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = shift(token_ptr);

	loop {
		if consume(token_ptr, "<") {
			node_ptr = new_node_calc(Nodekind::LThanNd, node_ptr, shift(token_ptr));

		} else if consume(token_ptr, "<=") {
			node_ptr = new_node_calc(Nodekind::LEqNd, node_ptr, shift(token_ptr));

		} else if consume(token_ptr, ">") {
			node_ptr = new_node_calc(Nodekind::GThanNd, node_ptr, shift(token_ptr));

		} else if consume(token_ptr, ">=") {
			node_ptr = new_node_calc(Nodekind::GEqNd, node_ptr, shift(token_ptr));

		} else{
			break;
		}
	}

	node_ptr

}


// 生成規則: shift = add ("<<" add | ">>" add)*
pub fn shift(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = add(token_ptr);

	loop {
		if consume(token_ptr, "<<") {
			node_ptr = new_node_calc(Nodekind::LShiftNd, node_ptr, add(token_ptr));

		} else if consume(token_ptr, ">>") {
			node_ptr = new_node_calc(Nodekind::RShiftNd, node_ptr, add(token_ptr));

		} else {
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
			node_ptr = new_node_calc(Nodekind::AddNd, node_ptr, mul(token_ptr));

		} else if consume(token_ptr, "-") {
			node_ptr = new_node_calc(Nodekind::SubNd, node_ptr, mul(token_ptr));

		} else {
			break;
		}
	}

	node_ptr
}

// 生成規則: mul = unary ("*" unary | "/" unary | "%" unary)*
fn mul(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = unary(token_ptr);

	loop {
		if consume(token_ptr, "*") {
			node_ptr = new_node_calc(Nodekind::MulNd, node_ptr, unary(token_ptr));

		} else if consume(token_ptr, "/") {
			node_ptr = new_node_calc(Nodekind::DivNd, node_ptr, unary(token_ptr));

		} else if consume(token_ptr, "%") {
			node_ptr = new_node_calc(Nodekind::ModNd, node_ptr, unary(token_ptr));

		} else {
			break;
		}
	}

	node_ptr
}

// TODO: *+x; *-y; みたいな構文を禁止したい
// 生成規則: unary = ("+" | "-")? primary | ("*" | "&") unary
fn unary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr;
	if consume(token_ptr, "*") {
		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::DerefNd,
				left: Some(unary(token_ptr)),
				..Default::default()}
		));

	} else if consume(token_ptr, "&") {
		node_ptr = Rc::new(RefCell::new(
			Node {
				kind: Nodekind::AddrNd,
				left: Some(unary(token_ptr)),
				..Default::default()
			}
		));

	} else if consume(token_ptr, "-") {
		// 単項演算のマイナスは0から引く形にする。
		node_ptr = new_node_calc(Nodekind::SubNd, new_node_num(0), primary(token_ptr));

	} else if consume(token_ptr, "+") {
		// 単項演算子のプラスは0に足す形にする。こうすることで &+var のような表現を generator 側で弾ける
		node_ptr = new_node_calc(Nodekind::AddNd, new_node_num(0), primary(token_ptr));

	} else {
		node_ptr = primary(token_ptr);
	}

	node_ptr
}

// 生成規則: primary = num | ident ( "(" (expr ",")* expr? ")" )? | "(" expr ")"
fn primary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr;

	if consume(token_ptr, "(") {
		node_ptr = expr(token_ptr);

		expect(token_ptr, ")");

	} else if is_ident(token_ptr) {
		let var_name = expect_ident(token_ptr);
		if consume(token_ptr, "(") {

			// 引数を6つまでサポート
			let mut args:Vec<Option<Rc<RefCell<Node>>>> = vec![];
			if !consume(token_ptr, ")") {
				// 引数が1つ以上あるパターン
				let mut argc: usize = 0;
				loop {
					if argc >= 6 {
						exit_eprintln!("現在7つ以上の引数はサポートされていません。");
					}
					if at_eof(token_ptr) {exit_eprintln!("関数呼び出しの\'(\'にマッチする\')\'が見つかりません。");}
					args.push(Some(expr(token_ptr)));
					argc += 1;

					// ','が読めたなら次の引数があるが、なければ引数列挙が終わらなければならない
					if !consume(token_ptr, ",") {
						expect(token_ptr, ")");
						break;
					}
				}
			}

			// 関数に対応するノード: あくまで今は外部とリンクさせて呼び出すため、関数を置くアドレスなどは気にしなくて良い
			node_ptr = Rc::new(RefCell::new(
				Node {
					kind: Nodekind::FuncNd,
					name: Some(var_name),
					args: args,
					..Default::default()
				}
			));

		} else {
			node_ptr = new_node_lvar(var_name);
		}
	} else {
		node_ptr = new_node_num(expect_number(token_ptr));
	}
	node_ptr
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::tokenizer::tokenize;

	static REP: usize = 40;

	fn search_tree(tree: &Rc<RefCell<Node>>) {
		let node: &Node = &*(*tree).borrow();
		println!("{}", node);

		if node.left.is_some() {search_tree(node.left.as_ref().unwrap());}
		if node.right.is_some() {search_tree(node.right.as_ref().unwrap());}
		if node.init.is_some() {search_tree(node.init.as_ref().unwrap());}
		if node.enter.is_some() {search_tree(node.enter.as_ref().unwrap());}
		if node.routine.is_some() {search_tree(node.routine.as_ref().unwrap());}
		if node.branch.is_some() {search_tree(node.branch.as_ref().unwrap());}
		if node.els.is_some() {search_tree(node.els.as_ref().unwrap());}
		for child in &node.children {
			if child.is_some() {search_tree(child.as_ref().unwrap());}
		}
		for arg in &node.args {
			if arg.is_some() {search_tree(arg.as_ref().unwrap());}
		}
		if node.stmts.is_some() {
			for stmt_ in node.stmts.as_ref().unwrap() {
				search_tree(stmt_);
			}
		}
	}


	pub fn parse_stmts(token_ptr: &mut Rc<RefCell<Token>>) -> Vec<Rc<RefCell<Node>>> {
	let mut statements :Vec<Rc<RefCell<Node>>> = Vec::new();
	while !at_eof(token_ptr) {
		statements.push(stmt(token_ptr));
	}

	statements
	}

	#[test]
	fn display() {
		println!("display{}", "-".to_string().repeat(REP));
		let node = new_node_num(0);
		println!("{}", (*node).borrow());
	}

	#[test]
	fn basic_calc() {
		println!("basic_calc{}", "-".to_string().repeat(REP));
		let equation = "
			x = 1 + 2 / 1;
			y = 200 % (3 + 1);
			z = 30 % 3 + 2 * 4;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn shift() {
		println!("shift{}", "-".to_string().repeat(REP));
		let equation = "
			x = 10 << 2 + 3 % 2 >> 3;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn bitops() {
		println!("bitops{}", "-".to_string().repeat(REP));
		let equation = "
			2 + (3 + 5) * 6;
			1 ^ 2 | 2 != 3 / 2;
			1 + -1 ^ 2;
			3 ^ 2 & 1 | 2 & 9;
			x = 10;
			y = &x;
			3 ^ 2 & *y | 2 & &x;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}


	#[test]
	fn logops() {
		println!("logops{}", "-".to_string().repeat(REP));
		let equation = "
			1 && 2 || 3 && 4;
			1 && 2 ^ 3 || 4 && 5 || 6;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn for_() {
		println!("for_{}", "-".to_string().repeat(REP));
		let equation = "
			sum = 10;
			sum = sum + i;
			for (i = 1 ; i < 10; i = i + 1) sum = sum +i;
			sum;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn while_() {
		println!("while_{}", "-".to_string().repeat(REP));
		let equation = "
			sum = 10;
			while(sum > 0) sum = sum - 1;
			sum;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn if_() {
		println!("if_{}", "-".to_string().repeat(REP));
		let equation = "
			i = 10;
			if (i == 10) i = i / 5;
			if (i == 2) i = i + 5; else i = i / 5;
			i;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}


	#[test]
	fn combination() {
		println!("combination{}", "-".to_string().repeat(REP));
		let equation = "
			i = 10;
			if (i == 10) i = i / 5;
			if (i == 2) i = i + 5; else i = i / 5;
			i;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn block() {
		println!("block{}", "-".to_string().repeat(REP));
		let equation = "
			for( i = 10; ; ) {i = i + 1;}
			{}
			{i = i + 1; 10;}
			return 10;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn block2() {
		println!("block2{}", "-".to_string().repeat(REP));
		let equation = "
			while(i < 10) {i = i + 1; i = i * 2;}
			x = 10;
			if ( x == 10 ){
				x = x + 200;
				x = x / 20;
			} else {
				x = x - 20;
				;
			}
			{{}}
			{i = i + 1; 10;}
			return 200;
			return;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn func() {
		println!("func{}", "-".to_string().repeat(REP));
		let equation = "
			call_fprint();
			i = getOne();
			j = getTwo();
			return i + j;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn func2() {
		println!("func2{}", "-".to_string().repeat(REP));
		let equation = "
			call_fprint();
			i = get(1);
			j = get(2, 3, 4);
			k = get(i+j, (i=3), k);
			return i + j;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn addr_deref() {
		println!("addr_deref{}", "-".to_string().repeat(REP));
		let equation = "
			x = 3;
			y = 5;
			z = &y + 8;
			return *z;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn addr_deref2() {
		println!("addr_deref2{}", "-".to_string().repeat(REP));
		let equation = "
			x = 3;
			y = &x;
			z = &y;
			return *&**z;
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn declare() {
		println!("declare{}", "-".to_string().repeat(REP));
		let equation = "
			func(x, y) {
				return x + y;
			}
			main() {
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
				return func(i, sum);
			}
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn no_return() {
		println!("declare{}", "-".to_string().repeat(REP));
		let equation = "
			func(x, y) {
				return x + y;
			}
			main() {
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
			}
		".to_string();
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}
}