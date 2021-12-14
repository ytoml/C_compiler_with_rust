use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::fmt;
use std::rc::Rc;

pub type TypeCellRef = Rc<RefCell<TypeCell>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Type {
	Invalid, // デフォルトや無名ノードに割り当てる
	Int,
	Char,
	Ptr,
	Func,
	Array,
}

impl Type {
	#[inline]
	pub fn bytes(&self) -> usize {
		match self {
			Type::Invalid => { panic!("cannot extract size of invalid type."); }
			Type::Char => { 1 }
			Type::Int => { 4 }
			Type::Ptr => { 8 }
			Type::Array => { panic!("cannot infer size of array from only itself"); }
			Type::Func => { panic!("access to the size of function should not be implemented yet"); }
		}
	}
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Type::Invalid => { s = "invalid"; }
			Type::Char => { s = "char"; }
			Type::Int => { s = "int"; }
			Type::Ptr => { s = "pointer"; }
			Type::Array => { s = "array"; }
			Type::Func => { s = "function"; }
		}
		write!(f, "{}", s)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawType {
	I8		= 0,
	I32		= 1, 
	U64		= 2, 
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

	// self.typ == Type::Func
	pub ret_typ: Option<TypeCellRef>,
	pub arg_typs: Option<Vec<TypeCellRef>>,
	pub is_unsigned: bool,
}

impl TypeCell {

	#[inline]
	pub fn new(typ: Type) -> Self {
		let is_unsigned = match typ {
			Type::Func | Type::Ptr => { true }
			_ => { false }
		};
		TypeCell { typ: typ, is_unsigned: is_unsigned, ..Default::default()}
	}

	#[inline]
	pub fn is_array(&self) -> bool {
		self.typ == Type::Array
	}
	
	#[inline]
	pub fn is_non_array(&self) -> bool {
		self.typ != Type::Array
	}

	#[inline]
	pub fn is_one_of(&self, types: &[Type]) -> bool {
		types.contains(&self.typ)
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
		TypeCell { typ: Type::Array, ptr_to: array_of, ptr_end: ptr_end, chains: chains, array_size: Some(size), ..Default::default() }
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

	#[inline]
	pub fn make_deref(&self) -> Result<Self, ()> {
		if self.is_one_of(&[Type::Array, Type::Ptr]) {
			Ok((*self.ptr_to.clone().unwrap().borrow()).clone())
		} else { Err(()) }
	}

	#[inline]
	pub fn get_base_cell(&self) -> Self {
		if let Some(_typ) = self.ptr_end {
			Self::new(_typ)
		} else {
			panic!("cannot extract base type from non-pointer.");
		}
	}

	#[inline]
	pub fn make_func(ret_typ: Self, arg_typs: Vec<TypeCellRef>) -> Self {
		let _ret_typ = Some(Rc::new(RefCell::new(ret_typ)));
		let _arg_typs = Some(arg_typs);
		TypeCell { typ: Type::Func, ret_typ: _ret_typ, arg_typs: _arg_typs, ..Default::default() }
	}

	#[inline]
	pub fn flatten_size(&self) -> usize {
		let (dim, _) = self.array_dim();
		dim.iter().product::<usize>()
	}

	#[inline]
	pub fn is_char_1d_array(&self) -> bool {
		self.typ == Type::Array && self.make_deref().unwrap().typ == Type::Char
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

	pub fn get_last_level_array(&self) -> Option<TypeCell> {
		let (dim, typ) = self.array_dim();
		if let Some(d) = dim.last() {
			Some(typ.make_array_of(*d))
		} else {
			None
		}
	}

	fn get_type_string(&self, s: impl Into<String>) -> String {
		let s = s.into();
		if let Some(deref) = &self.ptr_to {
			let string = if self.typ == Type::Array {
				if let Some(size) = self.array_size {
					format!("{}[{}]", s, size)
				} else {
					format!("{}[]", s)
				}
			} else if deref.borrow().typ == Type::Array {
				format!(" ({}*)", s)
			} else {
				format!("*{}", s)
			};
			(*deref).borrow().get_type_string(string)
		} else if self.typ == Type::Func {
			let ret_typ = self.ret_typ.as_ref().unwrap().borrow().clone();
			let mut args_str = String::new();
			for (ix, arg) in self.arg_typs.as_ref().unwrap().iter().enumerate() {
				args_str = if ix == 0 { format!("{}", arg.borrow()) } else { format!("{}, {}", args_str, arg.borrow()) };
			}
			format!("{} __func({}){}", ret_typ, args_str,s)
		} else {
			format!("{}{}", self.typ, s)
		}
	}
}

impl Default for TypeCell {
	fn default() -> Self {
		TypeCell { typ: Type::Invalid, ptr_to: None, ptr_end: None, chains: 0, array_size: None, arg_typs: None, ret_typ: None, is_unsigned: false }
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
			self.typ == other.typ && self.ret_typ == other.ret_typ && self.arg_typs == other.arg_typs

		}
	}
}

// 計算時、代入時などに暗黙のキャストを行うための処理
pub fn get_common_type(left_typ: TypeCell, right_typ: TypeCell) -> TypeCell {
	// 現在はポインタ、 int, char しかサポートされていない
	// 右側"のみ"がポインタになることはない(そのようなノード生成が起きる前にエラーになる)ことに注意
	if let Some(_typ) = left_typ.ptr_to {
		_typ.borrow().make_ptr_to()
	} else if let Some(_typ) = right_typ.ptr_to {
		_typ.borrow().make_ptr_to()
	} else {
		// int 以下のサイズの数は全て int にキャストされる
		TypeCell::new(Type::Int)
	}
}

pub fn get_raw_type(typ: Type) -> RawType {
	match typ {
		Type::Invalid => { panic!("cannot extract raw type from {}.", typ) }
		Type::Char => { RawType::I8 }
		Type::Int => { RawType::I32 }
		_ => { RawType::U64 }
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