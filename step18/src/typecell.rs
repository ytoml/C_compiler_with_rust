use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
	Invalid, // デフォルトや無名ノードに割り当てる
	Int,
	Ptr,
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Type::Invalid => { s = "invalid"; }
			Type::Int => { s = "int"; }
			Type::Ptr => { s = "pointer"; }
		}
		write!(f, "{}", s)
	}
}

#[derive(Clone, Debug, Eq)] // PartialEq は別で実装
pub struct TypeCell {
	pub typ: Type,
	pub ptr_to: Option<Rc<RefCell<TypeCell>>>,
}

impl TypeCell {
	pub fn new(typ: Type) -> Self {
		TypeCell { typ:typ, ptr_to: None }
	}

	fn get_ptr_chains(&self) -> (usize, Type) {
		match self.ptr_to.as_ref() {
			Some(ptr) => {
				let (p, typ) = ptr.borrow_mut().get_ptr_chains();
				(p+1, typ)
			}
			None => (1, self.typ.clone())
		}
	}
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell {typ: Type::Invalid, ptr_to: None}
	}
}

impl Display for TypeCell {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let s: &str;
		match self.typ {
			Type::Invalid => { s = "invalid"; }
			Type::Int => { s = "int"; }
			Type::Ptr => {
				let (p, typ) = self.ptr_to.as_ref().unwrap().borrow_mut().get_ptr_chains();
				return write!(f, "{}-chained pointer to {}", p, typ);
			}
		}
		write!(f, "{}", s)
	}
}

impl PartialEq for TypeCell {
	// ポインタが連なっている個数と、最終的に指されている型が両方同じ時にイコールとみなす
	fn eq(&self, other: &Self) -> bool {
		self.get_ptr_chains() == other.get_ptr_chains()
	}
}

unsafe impl Send for TypeCell {}
unsafe impl Sync for TypeCell {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn typecell_eq() {
		let mut t1 = TypeCell::new(Type::Int);
		let mut t2 = TypeCell::new(Type::Int);
		assert_eq!(t1, t2);

		for _ in 0..10 {
			let mut ptr =  TypeCell::new(Type::Ptr);
			let _ = ptr.ptr_to.insert(Rc::new(RefCell::new(t1)));
			t1 = ptr;
		}

		for _ in 0..11 {
			let mut ptr =  TypeCell::new(Type::Ptr);
			let _ = ptr.ptr_to.insert(Rc::new(RefCell::new(t2)));
			t2 = ptr;
		}

		assert_ne!(t1, t2);
		assert_eq!(t1, *t2.ptr_to.as_ref().unwrap().as_ref().borrow());
	}

}