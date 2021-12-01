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
	pub fn new(typ: &TypeCell, is_flex: bool) -> Self {
		match typ.typ {
			Type::Array => {
				if let Some(array_size) = typ.array_size {
					let mut elements = vec![];
					let elem_typ = typ.make_deref();
					for _ in 0..array_size {
						elements.push(Rc::new(RefCell::new(
							Initializer::new(&elem_typ, false)
						)));
					}
					Initializer { typ: Some(typ.clone()), elements: elements, ..Default::default() }
				} else {
					// flexible array
					if !is_flex { panic!("array flexibility conflicts."); }
					Initializer { typ: Some(typ.clone()), is_flex: true, ..Default::default() }
				}

			},
			_ => {
				Initializer { typ: Some(typ.clone()), ..Default::default() }
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
	}
}