// 再帰下降構文のパーサ
use crate::tokenizer::*;

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
	left: Option<Box<Node>>,
	right: Option<Box<Node>>,
	val: Option<i32>
}



fn new_node(kind: Nodekind, left: Box<Node>, right: Box<Node>) -> Box<Node> {
	let node_pointer = Box::new(
		Node {
			kind: kind,
			left: Some(left), 
			right: Some(right),
			val: None
		}

	);

	node_pointer
}

fn new_node_num(val: i32) -> Box<Node> {
	let node_pointer = Box::new(
		Node {
			kind: Nodekind::ND_NUM,
			left: None,
			right: None,
			val: Some(val)
		}
	);

	node_pointer
}


fn mul(token_stream: &Vec<Token>, index: &mut usize) -> Box<Node> {
	let mut node = primary(token_stream, index);

	loop {
		if consume(token_stream, index, "*") {
			node = new_node(Nodekind::ND_MUL, node, primary(token_stream, index));

		} else if consume(token_stream, index, "/") {
			node = new_node(Nodekind::ND_DIV, node, primary(token_stream, index));

		} else {
			break;
		}
	}

	node

}


fn expr(token_stream: &Vec<Token>, index: &mut usize) -> Box<Node> {
	let mut node = mul(token_stream, index);

	loop {
		if consume(token_stream, index, "+") {
			node = new_node(Nodekind::ND_ADD, node, mul(token_stream, index));

		} else if consume(token_stream, index, "-") {
			node = new_node(Nodekind::ND_SUB, node, mul(token_stream, index));

		} else {
			break;
		}

	}

	node
}

fn primary(token_stream: &Vec<Token>, index: &mut usize) -> Box<Node> {
	let mut node;
	if consume(token_stream, index, "(") {
		node = expr(token_stream, index);

		expect(token_stream, index, ")");

	} else {
		node = new_node_num(token_stream[*index].val.unwrap());

	}

	node
}
