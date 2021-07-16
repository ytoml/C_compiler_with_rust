// 再帰下降構文のパーサ
use crate::{exit_eprintln};
use crate::tokenizer::*;
use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::RefCell;

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


pub fn gen(node: &Rc<RefCell<Node>>, asm: &mut String) {
	if (**node).borrow().kind == Nodekind::ND_NUM {
		// ND_NUMの時点でunwrapできる
		*asm += format!("push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
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


fn mul(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node = primary(token_ptr);

	loop {
		if consume(token_ptr, "*") {
			node = new_node(Nodekind::ND_MUL, node, primary(token_ptr));

		} else if consume(token_ptr, "/") {
			node = new_node(Nodekind::ND_DIV, node, primary(token_ptr));

		} else {
			break;
		}
	}

	node

}


pub fn expr(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let mut node = mul(token_ptr);

	loop {
		if consume(token_ptr, "+") {
			node = new_node(Nodekind::ND_ADD, node, mul(token_ptr));

		} else if consume(token_ptr, "-") {
			node = new_node(Nodekind::ND_SUB, node, mul(token_ptr));

		} else {
			break;
		}

	}

	node
}

fn primary(token_ptr: &mut Rc<RefCell<Token>>) -> Rc<RefCell<Node>> {
	let node;
	if consume(token_ptr, "(") {
		node = expr(token_ptr);

		expect(token_ptr, ")");

	} else {
		node = new_node_num(*(**token_ptr).borrow().val.as_ref().unwrap());

	}

	node
}
