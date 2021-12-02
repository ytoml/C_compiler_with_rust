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
	pub fn flex_elem_count(&self) -> usize {
		if self.is_element() { panic!("invalid function on elemental initializer"); }
		let mut count = 0;
		// ネストされているものは1つとして読む
		for elem in self.elements.iter() {
			if !elem.borrow().is_element() {
				count += 1;
			} else { break; }
		}
		// 残りはそれぞれベース要素として読む
		let elem_size = self.elements[0].borrow().typ.clone().unwrap().flatten_size();
		let rem_count = ((self.elements.len() - count) + elem_size - 1)/ elem_size;

		count + rem_count
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
	}
}