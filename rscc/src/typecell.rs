use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Type {
	Invalid, // デフォルトや無名ノードに割り当てる
	Int,
	Ptr,
	Array,
}

impl Type {
	pub fn bytes(&self) -> usize {
		match self {
			Type::Invalid => { panic!("cannot extract size of invalid type."); }
			Type::Int => { 4 }
			Type::Ptr => { 8 }
			Type::Array => { panic!("cannot infer size of array from only itself"); }
		}
	}
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Type::Invalid => { s = "invalid"; }
			Type::Int => { s = "int"; }
			Type::Ptr => { s = "pointer"; }
			Type::Array => { s = "array"; }
		}
		write!(f, "{}", s)
	}
}

#[derive(Clone, Debug, Eq)] // PartialEq は別で実装
pub struct TypeCell {
	pub typ: Type,
	// ポインタの情報はいくつ繋がっているか及び終端の型で管理 (chains は *...*p の時の * の数)
	pub ptr_end: Option<Type>,
	pub chains: usize,

	pub array_size: Option<usize>,
}

impl TypeCell {
	pub fn new(typ: Type) -> Self {
		TypeCell { typ: typ, ptr_end: None, chains: 0, array_size: None}
	}
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell { typ: Type::Invalid, ptr_end: None, chains: 0, array_size: None}
	}
}

impl Display for TypeCell {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		if  let Some(typ) = &self.ptr_end {
			write!(f, "{} {}", &typ, "*".repeat(self.chains))
		} else {
			write!(f, "{}", &self.typ)
		}
	}
}

impl PartialEq for TypeCell {
	// ポインタが連なっている個数と、最終的に指されている型が両方同じ時にイコールとみなす
	fn eq(&self, other: &Self) -> bool {
		if let Some(typ) = &self.ptr_end {
			if let Some(other_typ) = &self.ptr_end {
				// この時点で両方ポインタなので typ のチェックは飛ばす
				self.chains == other.chains && typ == other_typ
			} else {
				false
			}
		} else {
			self.typ == other.typ
		}
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

		t1 = TypeCell {
			typ: Type::Ptr,
			ptr_end: Some(Type::Int),
			chains: 1,
			array_size: None,
		};
		assert_ne!(t1, t2);

		t2 = TypeCell {
			typ: Type::Ptr,
			ptr_end: Some(Type::Int),
			chains: 2,
			array_size: None
		};
		assert_ne!(t1, t2);

		t1 =  TypeCell {
			typ: Type::Ptr,
			ptr_end: Some(Type::Int),
			chains: 2,
			array_size: None
		};

		assert_eq!(t1, t2);
	}

}