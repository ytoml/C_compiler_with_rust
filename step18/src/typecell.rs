use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Type {
	Default,
	Int,
	Ptr,
}

impl Display for Type {
	fn fmt(&self, f:&mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Type::Default => { s = "none"; }
			Type::Int => { s = "int"; }
			Type::Ptr => { s = "pointer"; }
		}
		write!(f, "{}", s)
	}
}

pub struct TypeCell {
	pub ty: Type,
	pub ptr_to: Option<Rc<RefCell<TypeCell>>>,
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell {ty: Type::Default, ptr_to: None}
	}
}