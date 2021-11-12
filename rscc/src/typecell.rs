use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::fmt;
use std::rc::Rc;

pub type TypeCellRef = Rc<RefCell<TypeCell>>;

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
	// ポインタの情報はいくつ繋がっているか及び終端の型で管理 (chains は int *...*p; の時の * の数 + 配列の次元)
	// ポインタと配列を透過的に扱うために、配列の場合も ptr_to を持つ: ただし、ポインタの場合は ptr_end は Type::Ptr にならないが、配列の場合はあり得る
	// 配列の場合は、加えて array_size を持つ

	pub ptr_to: Option<TypeCellRef>,
	pub ptr_end: Option<Type>,
	pub chains: usize,
	pub array_size: Option<usize>,
}

impl TypeCell {
	pub fn new(typ: Type) -> Self {
		TypeCell { typ: typ, ..Default::default()}
	}

	pub fn make_ptr_to(&self) -> Self {
		let ptr_to = Some(Rc::new(RefCell::new(self.clone())));
		let ptr_end = Some(if let Some(end) = self.ptr_end { end } else { self.typ });
		let chains = self.chains + 1;
		TypeCell { typ: Type::Ptr, ptr_to: ptr_to, ptr_end: ptr_end, chains: chains, ..Default::default() }
	}

	// 配列は & と sizeof 以外に対してはポインタとして扱う
	// なので、ポインタと同じく ptr_end と chains も持たせておく(chains = dim(array) + chains(element))
	pub fn make_array_of(&self, size: usize) -> Self {
		let array_of = Some(Rc::new(RefCell::new(self.clone())));
		let ptr_end = if self.typ == Type::Array { self.ptr_end.clone() } else { Some(self.typ) };
		let chains = self.chains + 1;
		TypeCell { typ: Type::Array, ptr_to: array_of, ptr_end: ptr_end, chains: chains, array_size: Some(size) }
	}

	// 配列の次元と最小要素の型情報を取得
	pub fn array_dim(&self) -> (Vec<usize>, Self) {
		if let Some(size) = self.array_size {
			let element = self.ptr_to.clone(); // array_size が Some ならば必ず ptr_to も Some
			let (mut dim, typ) = (element.as_ref().unwrap()).borrow().array_dim();
			dim.insert(0, size);
			(dim, typ)
		} else {
			(vec![], self.clone())
		}
	}

	pub fn make_deref(&self) -> Self {
		if ![Type::Array, Type::Ptr].contains(&self.typ) { panic!("not able to extract element from non-array"); } 
		(*self.ptr_to.clone().unwrap().borrow()).clone()
	}

	pub fn bytes(&self) -> usize {
		match self.typ {
			Type::Array => {
				let (dim, typ) = self.array_dim();
				typ.typ.bytes() * dim.iter().product::<usize>()
			}
			_ => { self.typ.bytes() }
		}
	}

	fn get_type_string(&self, s: impl Into<String>) -> String {
		let s = s.into();
		if let Some(deref) = &self.ptr_to {
			let string = if let Some(size) = self.array_size {
				format!("{}[{}]", s, size)
			} else if deref.borrow().typ == Type::Array {
				format!("({}*)", s)
			} else {
				format!("*{}", s)
			};
			(*deref).borrow().get_type_string(string)
		} else {
			format!("{} {}", self.typ, s)
		}
	}
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell { typ: Type::Invalid, ptr_to: None, ptr_end: None, chains: 0, array_size: None}
	}
}

impl Display for TypeCell {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "{}", self.get_type_string(""))
	}
}

impl PartialEq for TypeCell {
	// ポインタが連なっている個数と、最終的に指されている型が両方同じ時にイコールとみなす
	// これは、配列とポインタを暗黙的に等価とみなすことにもなる
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
			..Default::default()
		};
		assert_ne!(t1, t2);

		t2 = TypeCell {
			typ: Type::Ptr,
			ptr_end: Some(Type::Int),
			chains: 2,
			..Default::default()
		};
		assert_ne!(t1, t2);

		t1 =  TypeCell {
			typ: Type::Ptr,
			ptr_end: Some(Type::Int),
			chains: 2,
			..Default::default()
		};

		assert_eq!(t1, t2);
	}

}