use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
	Invalid, // デフォルトや無名ノードに割り当てる
	Int,
	Ptr,
}

impl Display for Type {
	fn fmt(&self, f:&mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Type::Invalid => { s = "invalid"; }
			Type::Int => { s = "int"; }
			Type::Ptr => { s = "pointer"; }
		}
		write!(f, "{}", s)
	}
}

#[derive(Debug, Clone)]
pub struct TypeCell {
	pub typ: Type,
	pub ptr_to: Option<Rc<RefCell<TypeCell>>>,
}

impl TypeCell {
	pub fn new(typ: Type) -> Self {
		TypeCell { typ:typ, ptr_to: None }
	}

	pub fn set_ptr(&mut self, cell: TypeCell) {
		let _ = self.ptr_to.insert(Rc::new(RefCell::new(cell)));
	}
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell {typ: Type::Invalid, ptr_to: None}
	}
}

unsafe impl Send for TypeCell {}
unsafe impl Sync for TypeCell {}