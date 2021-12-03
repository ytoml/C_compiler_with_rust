use std::rc::Rc;
use std::cell::RefCell;

use crate::{
	node::NodeRef,
	typecell::TypeCell,
};

pub type InitializerRef = Rc<RefCell<Initializer>>;

#[derive(Clone, Debug, Default)]
pub struct Initializer {
	pub node:		Option<NodeRef>,		// 初期化する値に対応する式 
	pub typ:		Option<TypeCell>,		// タイプ
	pub elements:	Vec<InitializerRef>,	// 配列の各要素
	pub is_flex:	bool,					// 配列サイズを指定しない初期化
}

impl Initializer {
	pub fn new(typ: &TypeCell, node: &NodeRef) -> Self {
		Initializer { node: Some(Rc::clone(node)), typ: Some(typ.clone()), ..Default::default() }
	}

	#[inline]
	pub fn push_element(&mut self,  elem: Initializer) {
		self.elements.push(Rc::new(RefCell::new(elem)));
	}
	
	#[inline]
	pub fn is_element(&self) -> bool {
		self.elements.is_empty()
	}

	// 配列サイズを指定していない場合のサイズ特定を行う
	// parser::make_lvar_init と似たような処理
	pub fn flex_elem_count(&self) -> usize {
		if self.is_element() { panic!("invalid function on elemental initializer"); }
		let elem_flatten_size = self.elements[0].borrow().typ.as_ref().unwrap().flatten_size();
		let mut count = 0;
		let mut ix = 0;
		while ix < self.elements.len() {
			ix += if self.elements[ix].borrow().is_element() { elem_flatten_size } else { 1 };
			count += 1;
		}
		count
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
	}
}