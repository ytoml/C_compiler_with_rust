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
	ND_NUM
}

pub struct Node {
	pub kind: Nodekind,
	left: Option<Rc<RefCell<Node>>>,
	right: Option<Rc<RefCell<Node>>>,
	val: Option<i32>
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
	if (**node).borrow().kind == Nodekind::ND_NUM {
		// ND_NUMの時点でunwrapできる
		*asm += format!("	push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
		return;
	} 

	gen((**node).borrow().left.as_ref().unwrap(), asm);
	gen((**node).borrow().right.as_ref().unwrap(), asm);


	*asm += "	pop rdi\n";
	*asm += "	pop rax\n";

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
		_ => {
			exit_eprintln!();
		},
	}

	*asm += "	push rax\n";

}




fn new_node(kind: Nodekind, left: Rc<RefCell<Node>>, right: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
	let node_ptr = Rc::new(RefCell::new(
		Node {
			kind: kind,
			left: Some(left), 
			right: Some(right),
			val: None
		}

	));

	node_ptr
}

fn new_node_num(val: i32) -> Rc<RefCell<Node>> {
	let node_ptr = Rc::new(RefCell::new(
		Node {
			kind: Nodekind::ND_NUM,
			left: None,
			right: None,
			val: Some(val)
		}
	));

	node_ptr
}

pub fn expr(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = mul(token_ptr);

	// 生成規則: expr = mul ("+" mul | "-" mul)*
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

fn mul(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node_ptr = primary(token_ptr);

	// 生成規則: mul = primary ("*" mul | "/" mul)*
	loop {
		if consume(token_ptr, "*") {
			node_ptr = new_node(Nodekind::ND_MUL, node_ptr, primary(token_ptr));


		} else if consume(token_ptr, "/") {
			node_ptr = new_node(Nodekind::ND_DIV, node_ptr, primary(token_ptr));

		} else {
			break;
		}
	}


	node_ptr

}



fn primary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node_ptr;

	// 生成規則: primary = "(" expr ")" | num
	if consume(token_ptr, "(") {
		node_ptr = expr(token_ptr);

		expect(token_ptr, ")");

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
		println!("test_parser_brackets{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("(1+2)/3-1*20".to_string());
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}
}