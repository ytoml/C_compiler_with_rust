use std::rc::Rc;
use std::cell::RefCell;

use crate::{
	node::NodeRef,
	typecell::{Type, TypeCell},
};

pub type InitializerRef = Rc<RefCell<Initializer>>;

#[derive(Debug)]
pub struct Initializer {
	pub node:		Option<NodeRef>,		// 初期化する値に対応する式 
	pub typ:		Option<TypeCell>,		// タイプ
	pub elements:	Vec<InitializerRef>,	// 配列の各要素
	pub is_flex:	bool,					// 配列サイズを指定しない初期化
}

impl Default for Initializer {
	fn default() -> Initializer {
		Initializer {node: None, typ: None, elements: vec![], is_flex: false }
	}
}

impl Initializer {
	pub fn new(typ: &TypeCell, node: &NodeRef) -> Self {
		Initializer { node: Some(Rc::clone(node)), typ: Some(typ.clone()), ..Default::default() }
	}

	pub fn push_element(&mut self,  elem: Initializer) {
		self.elements.push(Rc::new(RefCell::new(elem)));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
	}
}