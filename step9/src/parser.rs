// 再帰下降構文のパーサ
use crate::{exit_eprintln};
use crate::tokenizer::*;
use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::{Formatter, Display, Result};

#[derive(Debug, PartialEq)]
pub enum Nodekind {
	ND_ADD,
	ND_SUB,
	ND_MUL,
	ND_DIV,
	ND_ASSIGN,
	ND_LVAR,
	ND_NUM,
	ND_EQ,
	ND_NE,
	ND_GT,
	ND_GE,
	ND_LT,
	ND_LE,
}

pub struct Node {
	pub kind: Nodekind,
	left: Option<Rc<RefCell<Node>>>,
	right: Option<Rc<RefCell<Node>>>,
	val: Option<i32>,
	offset: Option<usize> // ベースポインタからのオフセット(ローカル変数時のみ)
}

static REP_NODE:usize = 40;

impl Display for Node {
	fn fmt(&self, f:&mut Formatter) -> Result {

		writeln!(f, "{}", "-".to_string().repeat(REP_NODE));
		writeln!(f, "Nodekind : {:?}", self.kind);
		if let Some(e) = self.left.as_ref() {
			writeln!(f, "left: exist(kind:{:?})", (**self.left.as_ref().unwrap()).borrow().kind);
		} else {
			writeln!(f, "left: not exist");
		}

		if let Some(e) = self.left.as_ref() {
			writeln!(f, "right: exist(kind:{:?})", (**self.right.as_ref().unwrap()).borrow().kind);
		} else {
			writeln!(f, "right: not exist");
		}

		if let Some(e) = self.val.as_ref() {
			writeln!(f, "val: {}", e)
		} else {
			writeln!(f, "val: not exist")
		}
	}
}



pub fn gen(node: &Rc<RefCell<Node>>, asm: &mut String) {
	// 葉にきた、もしくは葉の親のところで左辺値にに何かしらを代入する操作がきた場合の処理
	match (**node).borrow().kind {
		Nodekind::ND_NUM => {
			// ND_NUMの時点でunwrapできる
			*asm += format!("	push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
			return;
		},
		Nodekind::ND_LVAR => {
			// 葉、かつローカル変数なので、あらかじめ代入した値へのアクセスを行う
			gen_lval(node, asm);
			*asm += "	pop rax\n"; // gen_lval内で対応する変数のアドレスをスタックにプッシュしているので、popで取れる
			*asm += "	mov rax, [rax]\n";
			*asm += "	push rax\n";
			return;
		},
		Nodekind::ND_ASSIGN => {
			// 節点、かつアサインゆえ左は左辺値の葉を想定(違えばgen_lval内でエラー)
			gen_lval((**node).borrow().left.as_ref().unwrap(), asm);
			gen((**node).borrow().right.as_ref().unwrap(), asm);

			// 上記gen2つでスタックに変数の値を格納すべきアドレスと、代入する値(式の評価値)がこの順で積んであるはずなので2回popして代入する
			*asm += "	pop rdi\n"; 
			*asm += "	pop rax\n"; 
			*asm += "	mov [rax], rdi\n";
			*asm += "	push rdi\n"; // 連続代入可能なように、評価値として代入した値をpushする
			return;
		},
		_ => {}// 他のパターンなら、ここでは何もしない
		
	} 

	gen((**node).borrow().left.as_ref().unwrap(), asm);
	gen((**node).borrow().right.as_ref().unwrap(), asm);

	*asm += "	pop rdi\n";
	*asm += "	pop rax\n";

	// >, >= についてはオペランド入れ替えのもとsetl, setleを使う
	match (**node).borrow().kind {
		Nodekind::ND_ADD => {
			*asm += "	add rax, rdi\n";
		},
		Nodekind::ND_SUB => {
			*asm += "	sub rax, rdi\n";
		},
		Nodekind::ND_MUL => {
			*asm += "	imul rax, rdi\n";
		},
		Nodekind::ND_DIV  => {
			*asm += "	cqo\n";
			*asm += "	idiv rdi\n";
		},
		Nodekind::ND_EQ => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	sete al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_NE => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setne al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_LT => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setl al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_LE => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setle al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_GT => {
			*asm += "	cmp rdi, rax\n";
			*asm += "	setl al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_GE => {
			*asm += "	cmp rdi, rax\n";
			*asm += "	setle al\n";
			*asm += "	movzb rax, al\n";
		},
		_ => {
			// 上記にないNodekindはここに到達する前にreturnしているはず
			exit_eprintln!("不正なNodekindです");
		},
	}

	*asm += "	push rax\n";

}

// 正しく左辺値を識別して不正な代入("(a+1)=2;"のような)を防ぐためのジェネレータ関数
fn gen_lval(node: &Rc<RefCell<Node>>, asm: &mut String) {
	if (**node).borrow().kind != Nodekind::ND_LVAR {
		exit_eprintln!("代入の左辺値が変数ではありません");
	}

	// 変数に対応するオフセットをスタックにプッシュする
	*asm += "	mov rax, rbp\n";
	*asm += format!("	sub rax, {}\n", (**node).borrow().offset.as_ref().unwrap()).as_str();
	*asm += "	push rax\n";
}


fn new_node(kind: Nodekind, left: Rc<RefCell<Node>>, right: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
	Rc::new(RefCell::new(
		Node {
			kind: kind,
			left: Some(left), 
			right: Some(right),
			val: None,
			offset: None
		}
	))
}

// 数字に対応するノード
fn new_node_num(val: i32) -> Rc<RefCell<Node>> {
	Rc::new(RefCell::new(
		Node {
			kind: Nodekind::ND_NUM,
			left: None,
			right: None,
			val: Some(val),
			offset: None
		}
	))
}

// 左辺値(今のうちはローカル変数)に対応するノード(現在は1文字のみ)
fn new_node_lvar(c: char) -> Rc<RefCell<Node>> {
	let offset = (c as usize - 'a' as usize)*8 +1;
	
	Rc::new(RefCell::new(
		Node {
			kind: Nodekind::ND_LVAR,
			left: None,
			right: None,
			val: None,
			offset: Some(offset)
		}

	))
}


// 生成規則: program = stmt*
fn program(token_ptr: &mut Rc<RefCell<Token>>) -> Vec<Rc<RefCell<Node>>> {
	let mut statements :Vec<Rc<RefCell<Node>>>= Vec::new();
	while !at_eof(token_ptr) {
		statements.push(stmt(token_ptr));
	}

	statements
}


//  生成規則: stmt = expr ";"
fn stmt(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {


	let node_ptr = expr(token_ptr);
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
		node_ptr = assign(token_ptr);
	}
	
	node_ptr
}

// 生成規則: equality = relational ("==" relational | "!=" relational)?
pub fn equality(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {

	let mut node_ptr = relational(token_ptr);
	loop {
		if consume(token_ptr, "==") {
			node_ptr = new_node(Nodekind::ND_EQ, node_ptr, relational(token_ptr));

		} else if consume(token_ptr, "!=") {
			node_ptr = new_node(Nodekind::ND_NE, node_ptr, relational(token_ptr));

		} else {
			break;
		}
	}

	node_ptr
}

// 生成規則: relational = add ("<" add | "<=" add | ">" add | ">=" add)*
fn relational(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = add(token_ptr);

	loop {
		if consume(token_ptr, "<") {
			node_ptr = new_node(Nodekind::ND_LT, node_ptr, add(token_ptr));

		} else if consume(token_ptr, "<=") {
			node_ptr = new_node(Nodekind::ND_LE, node_ptr, add(token_ptr));

		} else if consume(token_ptr, ">") {
			node_ptr = new_node(Nodekind::ND_GT, node_ptr, add(token_ptr));

		} else if consume(token_ptr, ">=") {
			node_ptr = new_node(Nodekind::ND_GE, node_ptr, add(token_ptr));

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
			node_ptr = new_node(Nodekind::ND_ADD, node_ptr, mul(token_ptr));

		} else if consume(token_ptr, "-") {
			node_ptr = new_node(Nodekind::ND_SUB, node_ptr, mul(token_ptr));

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
			node_ptr = new_node(Nodekind::ND_MUL, node_ptr, unary(token_ptr));

		} else if consume(token_ptr, "/") {
			node_ptr = new_node(Nodekind::ND_DIV, node_ptr, unary(token_ptr));

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
		node_ptr = new_node(Nodekind::ND_SUB, new_node_num(0), primary(token_ptr));

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
		let body = (**token_ptr).borrow_mut().body.as_ref().unwrap().clone();
		let cs:Vec<char> = body.chars().collect();
		node_ptr = new_node_lvar(cs[0]);

	} else {
		node_ptr = new_node_num(expect_number(token_ptr));
	}

	node_ptr
}


#[cfg(test)]
mod tests {
	use super::*;

	static REP:usize = 80;

	#[test]
	fn test_display() {
		println!("test_display{}", "-".to_string().repeat(40));
		let node = new_node_num(0);
		println!("{}", (*node).borrow());
	}


	#[test]
	fn test_parser_addsub() {
		println!("test_parser{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2+3-1".to_string());
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_parser_muldiv() {
		println!("test_parser{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2*3-4/2".to_string());
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_parser_brackets() {
		let equation = "(1+2)/3-1*20".to_string();
		println!("test_parser_brackets{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_parser_unary() {
		let equation = "(-1+2)*(-1)+(+3)/(+1)".to_string();
		println!("test_parser_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}
	
	#[test]
	fn test_parser_eq() {
		let equation = "(-1+2)*(-1)+(+3)/(+1) == 30 + 1".to_string();
		println!("test_parser_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}
	#[test]
	fn test_parser_assign() {
		let equation = "a = 1; a + 1;".to_string();
		println!("test_parser_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		let mut asm = "".to_string();
		for node_ptr in node_heads {
			gen(&node_ptr, &mut asm);

			asm += "	pop rax\n";
		}

		println!("{}", asm);

	}
}