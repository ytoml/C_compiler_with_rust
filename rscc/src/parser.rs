// 再帰下降構文のパーサ
use std::{cell::RefCell};
use std::collections::{HashMap, LinkedList};
use std::rc::Rc;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::{
	initializer::Initializer,
	node::{Node, Nodekind, NodeRef, InitData},
	token::{Tokenkind, TokenRef},
	tokenizer::{at_eof, consume, consume_ident, consume_kind, consume_literal, consume_number, consume_type, expect, expect_ident, expect_literal, expect_number, expect_type, is, is_func, is_kind, is_type},
	typecell::{Type, TypeCell, TypeCellRef, get_common_type},
	exit_eprintln, error_with_token, error_with_node
};

pub fn parse(mut head: TokenRef) -> Vec<NodeRef> {
	program(&mut head)
}

/// ローカル変数名 -> (BP からのオフセット,  型)
static LOCALS: Lazy<Mutex<Vec<HashMap<String, (usize, TypeCell)>>>> = Lazy::new(|| Mutex::new(vec![]));

/// グローバル変数名 -> 当該ノード
static GLOBALS: Lazy<Mutex<HashMap<String, Node>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static LVAR_MAX_OFFSET: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

/// 文字列 -> 対応する内部変数名
static LITERALS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// 単にリテラルのセクションの変数名を正しい順番で保持するためのリスト
pub static ORDERED_LITERALS: Lazy<Mutex<LinkedList<(String, String)>>> = Lazy::new(|| Mutex::new(LinkedList::new()));
static LITERAL_COUNT: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

fn store_literal(body: impl Into<String>) -> String {
	LITERALS.try_lock().unwrap()
		.entry(body.into())
		.or_insert_with_key(|body| {
			let name = format!(".LC{}", get_literal_id());
			ORDERED_LITERALS.try_lock().unwrap().push_back((body.clone(), name.clone()));
			name
		})
		.clone()
}

fn get_literal_id() -> usize {
	let mut count = LITERAL_COUNT.try_lock().unwrap();
	let c = *count;
	*count += 1;
	c
}

macro_rules! align {
	($addr:expr, $base:expr) => {
		if $base.count_ones() != 1 { panic!("invalid alignment basis: {}", $base); }
		$addr += $base - 1; 
		$addr &= !($base - 1)
	};
}

// 2つ子を持つ汎用ノード
#[inline]
fn _binary(kind: Nodekind, left: NodeRef, right: NodeRef, token: Option<TokenRef>) -> NodeRef {
	Rc::new(RefCell::new(Node{ kind: kind, token: token, left: Some(left), right: Some(right), .. Default::default()}))
}

#[inline]
fn new_binary(kind: Nodekind, left: NodeRef, right: NodeRef, token: TokenRef) -> NodeRef {
	_binary(kind, left, right, Some(token))
}

macro_rules! tmp_binary {
	($kind:expr, $left:expr, $right:expr) => {
		_binary($kind, $left, $right, None)
	};
}

// 1つ子を持つ汎用ノード
#[inline]
fn _unary(kind: Nodekind, left: NodeRef, token: Option<TokenRef>) -> NodeRef {
	Rc::new(RefCell::new(Node{ kind: kind, token: token, left: Some(left), .. Default::default()}))
}

#[inline]
fn new_unary(kind: Nodekind, left: NodeRef, token: TokenRef) -> NodeRef {
	_unary(kind, left, Some(token))
}

macro_rules! tmp_unary {
	($kind:expr, $left:expr) => {
		_unary($kind, Rc::clone(&($left)), None)
	};
}

// 数字に対応するノード
#[inline]
fn _num(val: i32, token: Option<TokenRef>) -> NodeRef {
	Rc::new(RefCell::new(Node{
		kind: Nodekind::NumNd,
		token: token,
		typ: Some(TypeCell::new(Type::Int)),
		val: Some(val),
		.. Default::default()
	}))
}

#[inline]
fn new_num(val: i32, token: TokenRef) -> NodeRef {
	_num(val, Some(token))
}

macro_rules! tmp_num {
	($num: expr) => {
		_num($num, None)
	};
}

fn get_alignment_base(typ: &TypeCell) -> usize {
	let bytes = typ.bytes();
	if bytes >= 16 { 16 }
	else if bytes >= 8 { 8 }
	else if bytes >= 4 { 4 }
	else if bytes >= 2 { 2 }
	else { 1 }
}

// 左辺値に対応するノード: += などの都合で無名の変数を生成する場合があるため、token は Option で受ける
fn _lvar(name: impl Into<String>, token: Option<TokenRef>, typ: Option<TypeCell>, is_local: bool, level: usize) -> NodeRef {
	if is_local {
		let (offset, _) = *LOCALS.try_lock().unwrap()[level]
			.entry(name.into().clone())
			.or_insert_with_key(|_|{
				// 見つからない場合にはオフセットの最大値を伸ばしてスタックを確保
				let mut max_offset_access  = LVAR_MAX_OFFSET.try_lock().unwrap();

				// 各変数のサイズ(配列なら1要素のサイズ)に alignment する
				let (diff, align_base) =  if let Some(t) = &typ { (t.bytes(), get_alignment_base(t)) } else { (8, 8) };
				*max_offset_access += diff;
				align!(*max_offset_access, align_base);

				// typ に渡されるのは Option だが LOCALS に保存するのは生の TypeCell であることに注意
				(*max_offset_access, if typ.is_some() { typ.clone().unwrap() } else { TypeCell::default() })
			});
		Rc::new(RefCell::new(Node{ kind: Nodekind::LvarNd, typ: typ, token: token, offset: Some(offset), is_local: is_local, level: Some(level), .. Default::default()}))
	} else {
		Rc::new(RefCell::new(Node{ kind: Nodekind::LvarNd, typ: typ, token: token, name: Some(name.into()), is_local: is_local, .. Default::default()}))
	}
}

#[inline]
fn new_lvar(name: impl Into<String>, token: TokenRef, typ: TypeCell, is_local: bool, level: usize) -> NodeRef {
	_lvar(name, Some(token), Some(typ), is_local, level)
}

macro_rules! tmp_lvar {
	() => {
		_lvar("", None, None, true, 0)
	};
}

// ブロックのノード
#[inline]
fn new_block(children: Vec<Option<NodeRef>>) -> NodeRef {
	Rc::new(RefCell::new(Node { kind: Nodekind::BlockNd, children: children, ..Default::default()}))
}

// 制御構文のためのノード
#[inline]
fn new_ctrl(kind: Nodekind,
			init: Option<NodeRef>,
			enter: Option<NodeRef>,
			routine: Option<NodeRef>,
			branch: Option<NodeRef>,
			els: Option<NodeRef>) -> NodeRef {
	if ![Nodekind::IfNd, Nodekind::ForNd, Nodekind::WhileNd].contains(&kind){
		exit_eprintln!("new_ctrl: 制御構文ではありません。");
	}
	Rc::new(RefCell::new(Node{ kind: kind, init: init, enter: enter, routine: routine, branch: branch, els: els, ..Default::default()}))
}

#[inline]
fn new_funcall(name: String, func_typ: TypeCell, args: Vec<Option<NodeRef>>, token: TokenRef) -> NodeRef {
	if func_typ.typ != Type::Func { panic!("new_funcall can be called only with function TypeCell"); }
	Rc::new(RefCell::new(Node{ kind: Nodekind::FunCallNd, token: Some(token), name: Some(name), func_typ:Some(func_typ), args: args, ..Default::default()}))
}

/// グローバル変数のノード(new_gvar, new_funcdec によりラップして使う)
#[inline]
fn _global(name: String, typ: Option<TypeCell>, func_typ: Option<TypeCell>, args: Vec<Option<NodeRef>>, stmts: Option<Vec<NodeRef>>, max_offset: Option<usize>, token: TokenRef) -> NodeRef {
	let glob = Rc::new(RefCell::new(Node{ kind: Nodekind::GlobalNd, token: Some(token), typ:typ, name: Some(name.clone()), func_typ: func_typ, args: args, stmts: stmts, max_offset: max_offset, ..Default::default() }));
	let _ = GLOBALS.try_lock().unwrap().insert(name.clone(), glob.borrow().clone());
	glob
}

#[inline]
fn new_gvar(name: String, typ: TypeCell, token: TokenRef) -> NodeRef {
	_global(name, Some(typ), None, vec![], None, None, token)
}

#[inline]
fn new_funcdec(name: String, func_typ: TypeCell, args: Vec<Option<NodeRef>>, stmts: Vec<NodeRef>, max_offset: usize, token: TokenRef) -> NodeRef {
	_global(name, None, Some(func_typ), args, Some(stmts), Some(max_offset), token)
}

#[inline]
fn proto_func(name: String, func_typ: TypeCell, token: TokenRef) -> NodeRef {
	_global(name, None, Some(func_typ), vec![], None, None, token)
}

#[inline]
fn nop() -> NodeRef {
	Rc::new(RefCell::new(Node{ kind: Nodekind::NopNd, typ: Some(TypeCell::new(Type::Invalid)), ..Default::default()}))
}

#[inline]
fn enter_scope() {
	LOCALS.try_lock().unwrap().push(HashMap::new());
}

#[inline]
fn leave_scope() {
	let _ = LOCALS.try_lock().unwrap().pop();
}

fn current_scope() -> usize {
	let n_scopes = LOCALS.try_lock().unwrap().len();
	if n_scopes == 0 { panic!("something wrong with scope, sufficient scope is not stacked.") }
	n_scopes - 1
}

/// 計算時の暗黙のキャストを行う関数
fn arith_cast(node: &mut Node) -> TypeCell {
	let left = Rc::clone(node.left.as_ref().unwrap());
	let right = Rc::clone(node.right.as_ref().unwrap());
	let left_typ = left.borrow().typ.clone().unwrap();
	let right_typ = right.borrow().typ.clone().unwrap();
	let typ = get_common_type(&left_typ, &right_typ);
	let _ = node.left.insert(new_cast(&left, typ.clone()));
	let _ = node.right.insert(new_cast(&right, typ.clone()));
	typ
}

fn new_cast(expr: &NodeRef, typ: TypeCell) -> NodeRef {
	confirm_type(&expr);
	let token = expr.borrow().token.clone();
	let left = Some(Rc::clone(expr));
	Rc::new(RefCell::new(Node { kind: Nodekind::CastNd, token: token, typ: Some(typ), left: left, ..Default::default() }))
}

/// 型を構文木全体に対して設定する関数
fn confirm_type(node: &NodeRef) {
	if let Some(_) = &node.borrow().typ { return; }

	if let Some(n) = &node.borrow().left { confirm_type(n); }
	if let Some(n) = &node.borrow().right { confirm_type(n); }
	if let Some(n) = &node.borrow().init { confirm_type(n); }
	if let Some(n) = &node.borrow().enter { confirm_type(n); }
	if let Some(n) = &node.borrow().routine { confirm_type(n); }
	if let Some(n) = &node.borrow().branch { confirm_type(n); }
	if let Some(n) = &node.borrow().els { confirm_type(n); }

	let kind: Nodekind = node.borrow().kind;
	let mut node = node.borrow_mut();
	match kind {
		Nodekind::NumNd => { let _ = node.typ.insert(TypeCell::new(Type::Int)); }
		Nodekind::AddrNd => {
			// & は変数やそのポインタにのみ可能であるため、このタイミングで left をチェックして弾くことができる
			let left_kind = node.left.as_ref().unwrap().borrow().kind;
			if ![Nodekind::DerefNd, Nodekind::LvarNd].contains(&left_kind) {
				error_with_node!("\"&\" では変数として宣言された値のみ参照ができます。", &node);
			}
			let left_typ = node.left.as_ref().unwrap().borrow().typ.clone().unwrap();
			let _ = node.typ.insert( left_typ.make_ptr_to() );
		}
		Nodekind::DerefNd => {
			let left_typ = node.left.as_ref().unwrap().borrow().typ.clone().unwrap();
			if let Some(_) = &left_typ.ptr_end {
				let _ = node.typ.insert( left_typ.make_deref().unwrap() );
			} else {
				error_with_node!("\"*\"ではポインタの参照を外すことができますが、型\"{}\"が指定されています。", &node, left_typ.typ);
			}
		}
		Nodekind::AssignNd => {
			// 右辺に関して暗黙のキャストを行う
			let left = node.left.as_ref().unwrap();
			let right = node.right.as_ref().unwrap();
			let left_typ = left.borrow().typ.clone().unwrap();
			
			if left_typ.is_array() {
				error_with_node!("左辺値は代入可能な型である必要がありますが、配列型\"{}\"が指定されています。", &left.borrow(), left_typ);
			}
			let right = new_cast(right, left_typ.clone());
			let _ = node.right.insert(right);
			let _ = node.typ.insert(left_typ);
		}
		Nodekind::AddNd | Nodekind::SubNd  => {
			// 暗黙のキャストを行う
			let typ = arith_cast(&mut node);
			let _ = node.typ.insert(typ);
		}
		Nodekind::BitNotNd => {
			// ポインタの bitnot は不可
			let left_typ = node.left.as_ref().unwrap().borrow().typ.clone().unwrap();
			if left_typ.ptr_end.is_some() {
				error_with_node!("ポインタのビット反転はできません。", &node);
			}
			let _ = node.typ.insert(left_typ);
		}
		Nodekind::MulNd | Nodekind::DivNd | Nodekind::ModNd |
		Nodekind::BitAndNd | Nodekind::BitOrNd | Nodekind::BitXorNd |
		Nodekind::LShiftNd | Nodekind::RShiftNd => {
			let typ = arith_cast(&mut node);
			if typ.ptr_end.is_some() {
				error_with_node!("ポインタに対して行えない計算です。", &node);
			}
			let _ = node.typ.insert(typ);
		}
		Nodekind::LogNotNd | Nodekind::LogAndNd | Nodekind::LogOrNd => {
			let _ = node.typ.insert(TypeCell::new(Type::Int));
		}
		Nodekind::EqNd | Nodekind::NEqNd | Nodekind::LThanNd | Nodekind::LEqNd => {
			let _ = arith_cast(&mut node);
			let _ = node.typ.insert(TypeCell::new(Type::Int));
		}
		Nodekind::CommaNd => {
			// x, y の評価は y になるため、型も y のものを引き継ぐ
			let right_typ = node.right.as_ref().unwrap().borrow().typ.clone().unwrap();
			let _ = node.typ.insert(right_typ);
		}
		Nodekind::FunCallNd => {
			// FunCallNd の func_typ.ret_typ を typ に適用することで自然に型を親ノードに伝播できる
			let ret_typ = node.func_typ.as_ref().unwrap().ret_typ.as_ref().unwrap().borrow().clone();
			let _ = node.typ.insert(ret_typ);
		}
		Nodekind::ReturnNd => {
			let left_typ = node.left.as_ref().unwrap().borrow().typ.clone().unwrap();
			let _ = node.typ.insert(left_typ);
		}
		Nodekind::ZeroClrNd => {
			let left_typ = node.left.as_ref().unwrap().borrow().typ.clone().unwrap();
			let _ = node.typ.insert(left_typ);
		}
		_ => {}
	}
}

/// 生成規則: 
/// program = global*
fn program(token_ptr: &mut TokenRef) -> Vec<NodeRef> {
	let mut globals : Vec<NodeRef> = Vec::new();
	while !at_eof(token_ptr) {
		enter_scope();
		globals.push(global(token_ptr));
		leave_scope();
		assert_eq!(LOCALS.try_lock().unwrap().len(), 0);

		// 関数宣言が終わるごとにローカル変数の管理情報をクリア(offset や name としてノードが持っているのでこれ以上必要ない)
		*LVAR_MAX_OFFSET.try_lock().unwrap() = 0;
	}
	globals
}

/// 生成規則:
/// global = function | global-variable
fn global(token_ptr: &mut TokenRef) -> NodeRef {
	let glob = 
	if is_func(token_ptr) {
		function(token_ptr)
	} else {
		global_variable(token_ptr)
	};
	glob
}

/// 生成規則: 
/// function = type ident "(" func-args ")" ("{" stmt* "}")?
fn function(token_ptr: &mut TokenRef) -> NodeRef {
	let mut typ = expect_type(token_ptr);
	let token = Rc::clone(token_ptr);
	let name = expect_ident(token_ptr);

	expect(token_ptr, "(");
	let (args, arg_typs) = func_args(token_ptr);
	let must_be_proto = args.len() != arg_typs.len();

	// 関数宣言の場合は typ は戻り値の型である
	typ = TypeCell::new(typ.typ).make_func(arg_typs);
	expect(token_ptr, ")");

	let (defined, line_num, line_offset) = 
	if let Some(node) = GLOBALS.try_lock().unwrap().get(&name) {
		let decl = node.token.as_ref().unwrap().borrow();
		let (_num, _offset) = (decl.line_num, decl.line_offset);
		if node.typ.is_some() { error_with_token!("\"{}\"は位置[{}, {}]で既にグローバル変数として宣言されています。", &*token.borrow(), name, _num, _offset); }
		(node.stmts.is_some(), _num, _offset)
	} else { (false , 0, 0) };

	if consume(token_ptr, "{") {
		if must_be_proto { error_with_token!("関数の定義時には引数名を省略できません。", &*token.borrow()); }
		// 既に宣言されている場合をケア
		let node = GLOBALS.try_lock().unwrap().get(&name).cloned().unwrap_or(Node::default());
		match node.kind {
			Nodekind::GlobalNd => {
				if defined { error_with_token!("関数\"{}\"は位置[{}, {}]で既に定義義されています。", &*token.borrow(), name, line_num, line_offset); }
				// プロトタイプ宣言時と引数の整合をチェック
				if typ != *node.func_typ.as_ref().unwrap(){ error_with_token!("プロトタイプ宣言との互換性がありません。(宣言位置: [{}, {}])", &*token.borrow(), line_num, line_offset); }
			}
			Nodekind::DefaultNd => {
				// プロトタイプ宣言がない場合は、再帰のことを考えて定義のパース前に GLOBALS に一旦プロトタイプ宣言の体で保存する
				let _ = proto_func(name.clone(), typ.clone(), Rc::clone(&token));
			}
			_ => { panic!("unreachable"); }
		}

		let mut stmts : Vec<NodeRef> = Vec::new();
		let mut has_return : bool = false;
		while !consume(token_ptr, "}") {
			has_return |= token_ptr.borrow().kind == Tokenkind::ReturnTk; // return がローカルの最大のスコープに出現するかどうかを確認 (ブロックでネストされていると対応できないのが難点…)
			let stmt_ = stmt(token_ptr);
			confirm_type(&stmt_);
			stmts.push(stmt_);
		}

		if !has_return {
			stmts.push(tmp_unary!(Nodekind::ReturnNd, tmp_num!(0)));
		}

		let mut max_offset_access = LVAR_MAX_OFFSET.try_lock().unwrap();
		align!(*max_offset_access, 8usize);
		let max_offset = *max_offset_access;

		new_funcdec(name, typ, args, stmts, max_offset, token)

	} else {
		expect(token_ptr, ";");
		let _ = proto_func(name, typ, token);
		nop()
	}
}

/// 生成規則:
/// func-args = arg ("," arg)* | null
/// arg = type ident?
fn func_args(token_ptr: &mut TokenRef) -> (Vec<Option<NodeRef>>, Vec<TypeCellRef>) {
	let mut args: Vec<Option<NodeRef>> = vec![];
	let mut arg_typs: Vec<TypeCellRef> = vec![];
	let mut argc: usize = 0;
	if let Some(typ) = consume_type(token_ptr) { // 型宣言があれば、引数ありと判断
		arg_typs.push(Rc::new(RefCell::new(typ.clone())));

		let token = Rc::clone(token_ptr);
		if let Some(name) = consume_ident(token_ptr) {
			args.push(Some(new_lvar(name, token, typ.clone(), true, 0)));
		}
		argc += 1;

		loop {
			if !consume(token_ptr, ",") {break;}
			if argc >= 6 {
				error_with_token!("現在7つ以上の引数はサポートされていません。", &token_ptr.borrow());
			}
			let typ = expect_type(token_ptr); // 型宣言の読み込み
			arg_typs.push(Rc::new(RefCell::new(typ.clone())));

			let token = Rc::clone(token_ptr);
			if let Some(name) = consume_ident(token_ptr) {
				args.push(Some(new_lvar(name, token, typ.clone(), true, 0)));
			}
			argc += 1;
		}
	} else {
		// エラーメッセージがわかりやすくなるように分岐する
		let token = Rc::clone(token_ptr);
		if let Some(_) = consume_ident(token_ptr) {
			error_with_token!("型指定が必要です。", &*token.borrow());
		}
	}
	// args.len() != arg_types.len() ならば引数名が省略されており、プロトタイプ宣言であるとみなせる
	(args, arg_typs)
}

/// 生成規則:
/// global-variable = type gvar-decl ("," gvar-decl)* ";"
fn global_variable(token_ptr: &mut TokenRef) -> NodeRef {
	let typ = expect_type(token_ptr);
	let mut node_ptr = gvar_decl(token_ptr, typ.clone());
	loop {
		let comma_token = Rc::clone(token_ptr);
		if !consume(token_ptr, ",") { break; }
		node_ptr = new_binary(Nodekind::CommaNd, node_ptr, gvar_decl(token_ptr, typ.clone()), comma_token)
	}
	expect(token_ptr,";");
	
	node_ptr
}

/// 生成規則:
/// gvar-decl = ident ("[" array-suffix)? ("=" initializer)?
fn gvar_decl(token_ptr: &mut TokenRef, mut typ: TypeCell) -> NodeRef {
	let token = Rc::clone(token_ptr);
	let name = expect_ident(token_ptr);
	if let Some(node) = GLOBALS.try_lock().unwrap().get(&name) {
		let decl = node.token.as_ref().unwrap().borrow();
		if node.typ.is_some() {
			error_with_token!("\"{}\"は位置[{}, {}]で既にグローバル変数として宣言されています。", &*token.borrow(), name, decl.line_num, decl.line_offset);
		} else {
			error_with_token!("\"{}\"は位置[{}, {}]で既に関数として宣言されています。", &*token.borrow(), name, decl.line_num, decl.line_offset);
		}
	}

	let mut is_flex = false;
	if consume(token_ptr, "[") {
		is_flex = array_suffix(token_ptr, &mut typ);
	}

	let gvar =
	if consume(token_ptr, "=") {
		gvar_initializer(token_ptr, name, typ, is_flex, token)
	} else {
		new_gvar(name, typ, token)
	};
	gvar
}

/// 生成規則としては lvar_initializer と同じ
fn gvar_initializer(token_ptr: &mut TokenRef, name: String, typ: TypeCell, is_flex: bool, token: TokenRef) -> NodeRef {
	let mut typ = typ.clone();
	if typ.is_array() && !is_kind(token_ptr, Tokenkind::StringTk) && !is(token_ptr, "{") { error_with_token!("配列の初期化の形式が異なります。", &token_ptr.borrow()); }
	if typ.array_dim().0.len() > 1 && is_kind(token_ptr, Tokenkind::StringTk) {
		error_with_token!("2次元以上の配列\"{}\"は単一の文字リテラルでは初期化できません。", &*token_ptr.borrow(), typ);
	}

	let init = initializer(token_ptr, typ.clone());
	if is_flex {
		let _ = typ.array_size.insert(init.flex_elem_count());
	}

	let mut gvar = new_gvar(name, typ.clone(), token);
	make_gvar_init(&init, typ, &mut gvar);
	gvar
}

fn make_gvar_init(init: &Initializer, typ: TypeCell, gvar: &mut NodeRef) {
	if typ.is_array() {
		let total_bytes = typ.bytes();
		let elem_typ = typ.make_deref().unwrap();
		let elem_bytes = elem_typ.bytes();
		let elem_flatten_size = elem_typ.flatten_size();
		let base_typ = typ.get_base_cell();
		let base_bytes = base_typ.bytes();
		let mut ix = 0;
		let mut finished_bytes = 0;
		while finished_bytes < total_bytes && ix < init.elements.len() {
			let elem = &init.elements[ix];
			if elem.borrow().is_element() {
				// flatten して読む
				for _ in 0..elem_flatten_size {
					let _expr = init.elements[ix].borrow().node.clone().unwrap();
					let mut label: Option<String> = None;
					let val = eval_const(&_expr, &mut label);
					gvar.borrow_mut().init_data.push(InitData::new(base_bytes, val, label));
					ix += 1;
					finished_bytes += base_bytes;
					if finished_bytes >= total_bytes || ix >= init.elements.len() { break; }
				}
				
			} else {
				make_gvar_init(&elem.borrow(), elem_typ.clone(), gvar);
				ix += 1;
				finished_bytes += elem_bytes;
			}
			// 毎要素、後ろの0をまとめる
			let mut gvar_bor = gvar.borrow_mut();
			let init_data = &mut gvar_bor.init_data;
			let last_data = init_data.last().unwrap();
			let mut zero_end = last_data.val == 0 && last_data.label.is_none();
			if zero_end {
				let mut zeros = InitData::new(0, 0, None);
				while zero_end && init_data.len() > 0 {
					let merge_data = init_data.pop().unwrap();
					zeros.size += merge_data.size;
					if let Some(last_data) = init_data.last() {
						zero_end = last_data.val == 0 && last_data.label.is_none();
					}
				}
				init_data.push(zeros);
			}
		}
		// 初期化値が指定されていない残りの要素を0埋め
		if finished_bytes < total_bytes { gvar.borrow_mut().init_data.push(InitData::new(total_bytes - finished_bytes, 0, None)); }
	} else {
		let mut label: Option<String> = None;
		let val = eval_const(init.node.as_ref().unwrap(), &mut label);
		gvar.borrow_mut().init_data.push(InitData::new(typ.bytes(), val, label));
	}
} 

macro_rules! eval_const_left {
	($node: expr, $label: expr) => {
		eval_const(($node).borrow().left.as_ref().unwrap(), ($label))
	};
}

macro_rules! eval_const_right {
	($node: expr, $label: expr) => {
		eval_const(($node).borrow().right.as_ref().unwrap(), ($label))
	};
}

/// コンパイル時定数の処理を行う関数
/// label はグローバル変数への参照があった場合にどの変数を参照しているかを持つ
fn eval_const(node: &NodeRef, label: &mut Option<String>) -> i64 {
	confirm_type(node);
	let typ = node.borrow().typ.clone().unwrap();
	let kind = node.borrow().kind;
	match kind {
		Nodekind::AddNd		=> { eval_const_left!(node, label) + eval_const_right!(node, label) }
		Nodekind::SubNd		=> { 
			let left_val = eval_const_left!(node, label);
			let left_label = label.clone();
			let right_val = eval_const_right!(node, label);
			match (left_label, &label) {
				(Some(l), Some(r)) => {
					// コンパイル時のポインタ同士の引き算は、同じラベル同士でのみ可能(打ち消し合うのでラベルをクリアする)
					if l != *r { error_with_node!("コンパイル時定数のみが使用可能です。", &node.borrow().right.as_ref().unwrap().borrow()); }
					let _ = label.take();
				}
				_ => {}
			}
			left_val - right_val
		}
		Nodekind::MulNd		=> { eval_const_left!(node, label) * eval_const_right!(node, label) }
		Nodekind::DivNd		=> { eval_const_left!(node, label) / eval_const_right!(node, label) }
		Nodekind::ModNd		=> { eval_const_left!(node, label) % eval_const_right!(node, label) }
		Nodekind::LShiftNd	=> { eval_const_left!(node, label) << eval_const_right!(node, label) }
		Nodekind::RShiftNd	=> { eval_const_left!(node, label) >> eval_const_right!(node, label) }
		Nodekind::BitAndNd	=> { eval_const_left!(node, label) & eval_const_right!(node, label) }
		Nodekind::BitOrNd	=> { eval_const_left!(node, label) | eval_const_right!(node, label) }
		Nodekind::BitXorNd	=> { eval_const_left!(node, label) ^ eval_const_right!(node, label) }
		Nodekind::BitNotNd	=> { !eval_const_left!(node, label) }
		Nodekind::LogAndNd	=> { if eval_const_left!(node, label) == 0 || eval_const_right!(node, label) == 0 { 0 } else { 1 } }
		Nodekind::LogOrNd	=> { if eval_const_left!(node, label) == 0 && eval_const_right!(node, label) == 0 { 0 } else { 1 } }
		Nodekind::LogNotNd	=> { if eval_const_left!(node, label) == 0 { 1 } else { 0 } }
		Nodekind::CastNd	=> {
			let val = eval_const_left!(node, label);
			match typ.typ.bytes() {
				1 => { if typ.is_unsigned { val as	u8 as i64 } else { val as  i8 as i64 }}
				2 => { if typ.is_unsigned { val as u16 as i64 } else { val as i16 as i64 }}
				4 => { if typ.is_unsigned { val as u32 as i64 } else { val as i32 as i64 }}
				_ => { val }
			}
		}
		Nodekind::AddrNd	=> { eval_label(node.borrow().left.as_ref().unwrap(), label) }
		Nodekind::NumNd		=> { node.borrow().val.unwrap() as i64 }
		_ => { error_with_node!("コンパイル時定数のみが使用可能です。", &node.borrow()); }
	}
}

/// グローバル変数のアドレス評価時などに使用する関数
fn eval_label(node: &NodeRef, label: &mut Option<String>) -> i64 {
	let kind = node.borrow().kind;
	match kind {
		Nodekind::DerefNd	=> { eval_const_left!(node, label) }
		Nodekind::LvarNd	=> {
			// 初期化時に仮で生成される変数はグローバルスコープでも(!is_local な) LvarNd であることに注意
			// また、 Initializer のパース時に定義されていないグローバル変数は弾かれるため、ここでは宣言チェック不要
			if node.borrow().is_local { error_with_node!("コンパイル時定数のみが使用可能です。", &node.borrow()); }
			let _ = label.insert(node.borrow().name.clone().unwrap());
			0
		} 
		_ => { error_with_node!("コンパイル時定数のみが使用可能です。", &node.borrow()); }
	}
}

/// 生成規則:
/// declarator = pointers ("(" declarator ")" | ident ) type-suffix
fn declarator(token_ptr: &mut TokenRef, mut typ: TypeCell) -> (String, TypeCell) {
	typ = pointers(token_ptr, typ);
	

	(String::new(), TypeCell::default())
}

/// 生成規則: 
/// pointers = ("*")*
fn pointers(token_ptr: &mut TokenRef, mut typ: TypeCell) -> TypeCell {
	while consume(token_ptr, "*") {
		typ = typ.make_ptr_to();
	}
	typ
}

/// 生成規則: 
/// type-suffix = "(" func-args | "[" array-suffix | null
fn type_suffix(token_ptr: &mut TokenRef, mut typ: TypeCell) -> TypeCell {
	if consume(token_ptr, "(") {
		let (_, arg_typs) = func_args(token_ptr);
		typ.make_func(arg_typs)
	} else if consume(token_ptr, "[") {
		let is_flex = array_suffix(token_ptr, &mut typ);
		typ
	} else {
		typ
	}
}

/// 生成規則:
/// declaration = type lvar-decl ("," lvar-decl )* ";"
fn declaration(token_ptr: &mut TokenRef) -> NodeRef {
	let typ = expect_type(token_ptr);
	let mut node_ptr = lvar_decl(token_ptr, typ.clone());
	loop {
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, ",") { break; }
		node_ptr = new_binary(Nodekind::CommaNd, node_ptr, lvar_decl(token_ptr, typ.clone()), token)
	}
	expect(token_ptr,";");
	
	node_ptr
}

/// 生成規則:
/// lvar-decl = ident ("[" array-suffix)? ("=" initializer)?
fn lvar_decl(token_ptr: &mut TokenRef, mut typ: TypeCell) -> NodeRef {
	let token = Rc::clone(token_ptr);
	let name = expect_ident(token_ptr);
	if LOCALS.try_lock().unwrap().last().unwrap().contains_key(&name) { error_with_token!("既に宣言された変数です。", &token.borrow()); }

	let mut is_flex = false;
	if consume(token_ptr, "[") {
		is_flex = array_suffix(token_ptr, &mut typ);
	}

	if consume(token_ptr, "=") {
		lvar_initializer(token_ptr, name, typ, is_flex, token)
	} else {
		// 初期化しない場合は何もアセンブリを吐かない
		if is_flex { error_with_token!("初期化しない場合は完全な配列サイズが必要です。", &token.borrow()); }
		let _ = new_lvar(name, token, typ, true, current_scope());
		nop()
	}
}

/// 生成規則:
/// array-suffix = num "]" ("[" array-suffix)?
fn array_suffix(token_ptr: &mut TokenRef, typ: &mut TypeCell) -> bool {
	let ptr_err = Rc::clone(token_ptr);
	let (size, is_flex) = if let Some(num) = consume_number(token_ptr) { (num as usize, false) } else { (0, true) };
	if consume(token_ptr, "-") { error_with_token!("配列のサイズは0以上である必要があります。", &ptr_err.borrow()); }
	expect(token_ptr, "]");

	// 配列の次元は後ろから処理する
	if consume(token_ptr, "[") {
		let ptr_err = Rc::clone(token_ptr);
		if consume(token_ptr, "]") { error_with_token!("2次元目以降の要素サイズは必ず指定する必要があります。", &ptr_err.borrow()); }
		let _ = array_suffix(token_ptr, typ);
	}

	// flex な場合は　array_size を None にする
	*typ = typ.make_array_of(size);
	if is_flex { let _ = typ.array_size.take(); }
	is_flex
}

/// 規則 initializer により Initializer を生成し、AssignNd による代入へと変換する
fn lvar_initializer(token_ptr: &mut TokenRef, name: String, mut typ: TypeCell, is_flex: bool, token: TokenRef) -> NodeRef {
	if typ.is_array() && !is_kind(token_ptr, Tokenkind::StringTk) && !is(token_ptr, "{") { error_with_token!("配列の初期化の形式が異なります。", &token_ptr.borrow()); }
	if typ.array_dim().0.len() > 1 && is_kind(token_ptr, Tokenkind::StringTk) {
		error_with_token!("2次元以上の配列\"{}\"は単一の文字リテラルでは初期化できません。", &*token_ptr.borrow(), typ);
	}

	let init = initializer(token_ptr, typ.clone());
	if is_flex {
		let _ = typ.array_size.insert(init.flex_elem_count());
	}

	let lvar = new_lvar(name, Rc::clone(&token), typ.clone(), true, current_scope());
	let offset = lvar.borrow().offset.unwrap();
	match typ.typ {
		Type::Array => {
			let zero_clear = new_unary(
				Nodekind::ZeroClrNd,
				lvar,
				Rc::clone(&token)
			);
			new_binary(
				Nodekind::CommaNd,
				zero_clear,
				make_lvar_init(init, typ, offset, false, Rc::clone(&token)),
				token
			)
		}
		_ => {
			make_lvar_init(init, typ, offset, true, token)
		}
	}
}

/// 生成規則:
/// initializer = "{" array-initializer | char-array-initializer | assign
fn initializer(token_ptr: &mut TokenRef, typ: TypeCell) -> Initializer {
	// char の1次元配列のみ文字列リテラルで初期化できるため、特別扱い
	if typ.is_char_1d_array() {
		// string-literal か "{" string-literal "}" の形であれば char-array-initializer を呼ぶ(トークンの先読みが必要なため、clone してから読んでいることに注意)
		let mut _token_ptr = Rc::clone(token_ptr);
		let braced = consume(&mut _token_ptr, "{");
		let token = Rc::clone(&_token_ptr);
		if let Some(body) = consume_literal(&mut _token_ptr) {
			if braced { expect(token_ptr, "{"); }
			let _ = expect_literal(token_ptr);
			let init = char_array_initializer(body, typ.array_size, token);
			let _ = consume(token_ptr, ",");
			if braced && !consume(token_ptr, "}") { error_with_token!("char の1次元配列を文字列リテラルで初期化する場合は1つのみ配置してください。", &token_ptr.borrow()); }
			return init;
		}
	} 

	let mut init = Initializer::default();
	if consume(token_ptr, "{") {
		if typ.is_non_array() {
			// スカラ値に代入することになるため、最初の要素以外読み飛ばす
			let mut _init = Initializer::default();
			let arr_init = array_initializer(token_ptr, typ.clone());
			init.insert(typ, Rc::clone(arr_init.node.as_ref().unwrap()));
			init
		} else {
			array_initializer(token_ptr, typ)
		}
	} else {
		if is_kind(token_ptr, Tokenkind::StringTk) && typ.is_array() && !typ.make_deref().unwrap().is_one_of(&[Type::Char, Type::Ptr, Type::Array]) {
			error_with_token!("文字列リテラルで\"{}\"型の変数を初期化することはできません", &*token_ptr.borrow(), typ);
		}
		init.insert(typ, assign(token_ptr));
		init
	}
} 

/// 生成規則:
/// char-array-initializer = string-literal
fn char_array_initializer(body: String, array_size: Option<usize>, token: TokenRef) -> Initializer {
	let mut init = Initializer::default();
	let elems = body.as_bytes().iter().map(|c| *c as i32);
	let elem_typ = TypeCell::new(Type::Char);
	let size = 
	if let Some(_size) = array_size {
		// 配列は、どんな型であれ初期値の指定がない箇所は0で初期化されるため、固定長の場合は終端'\0'としての (int)0 を生成するノードは不要
		let mut ix: usize = 0;
		for e in elems {
			if ix >= _size { break; }
			ix += 1;
			init.push_element(Initializer::new(elem_typ.clone(), new_num(e, Rc::clone(&token))));
		}
		while ix < _size {
			ix += 1;
			// 0 パディング
			init.push_element(Initializer::new(elem_typ.clone(), new_num(0, Rc::clone(&token))));
		}
		_size
	} else {
		for e in elems{
			init.push_element(Initializer::new(elem_typ.clone(), new_num(e, Rc::clone(&token))));
		}
		init.push_element(Initializer::new(elem_typ.clone(), new_num(0, Rc::clone(&token))));
		init.elements.len()
	};
	// この関数が呼ばれている時点でネストが深すぎるということはないため、ここで持たせる node はなんでも良い
	init.insert(elem_typ.make_array_of(size), new_num(0, token));
	init
}

// 配列の初期化について
// 要素数が足りない時:
// "int x[3] = {1, 2};" -> x[0] = 1, x[1] = 2, x[2] = 0;
// - これは単に0埋めである
// 
// ネストが浅すぎる時:
// "int x[2][2] = {1, 2};" or "int x[2][2] = {1, {2, 1}}" -> x[0][0] = 1, x[0][1] = 2;
// - これは opening brace で始まらない要素があると、そこからそのレベルの1要素におけるベース型の格納個数だけ要素を読むという処理に起因している
// - 例えば、 int x[4][2][1] = {1, {2, 3}, 4, 5, {6}, 7, 8, 9}; は
// - int x[4][2][1] = {{{1}, {2}}, {{4}, {5}}, {{6}, {0}}, {{7}, {8}}}; と同じ
// 
// ネストが深すぎる時:
// "int x[2][2] = {{{1, 2, 3}, 10}, 20};" -> x[0][0] = 1, x[0][1] = 10, x[1][0] = 20;
// - これは、それ以上の sub-array がない場合には先頭の要素のみを扱うことになっている
// - 例えば、 int x = {{2, 3}, 4}; なども valid であり、これは単に int x = 2; と同じ
// 
// 文字列リテラルによる初期化のルール
// [`char str[] = "abc"; と char str[] = {"abc"};`] は char str[] = {'a', 'b', 'c', '\0'}; と同じ(1つめが例外的表現)
// 「ネストが深すぎる時」に該当する場合を除き、文字列リテラルを char 配列以外の初期化に使用することはできない
// また、2次以上の配列を中括弧なしの文字列で初期化することはできない
// 
// ネストが浅すぎる時: 
// char str[]~[2] = {"abc~", "~", ...}; のようなパターンだと、最下位レベルの配列要素の数まで各リテラルを打ち切り、各リテラルと同じレベルに展開する
// - 例えば、 char str[][2][2] = {"abc", "def", "ghi"}; とするとこの初期化は {'a', 'b', 'd', 'e', 'g', 'h'} と同じ
// - この時、 {{'a', 'b'}, {'d', 'e'}, {'g', 'h'}} とはならないことに注意
// - よって、基本的な初期化のルールに従って char str[2][2][2] = {{{'a', 'b'}, {'d', 'e'}}, {{'g', 'h'}, {}}}; と同様の初期化であると解釈される
// 
// ネストが深すぎる時: 
// 単にその文字列リテラルへのポインタを要素として代入することになり、冗長な要素の読み飛ばしは基本的なネストのルールに従う
// - 例えば、 char str[][2] = {{{"abc"}}, "def"}; は {{"abc", 0}, 'd', 'e'} すなわち {{(char)&.LC0, 0}, {'d', 'e'}} である
// - これは make_lvar_init など Initializer を Node に変換する時に処理するものとする
// 
// また、char[] を文字列で初期化する場合に、例えば
// char str[] = {"abc", "def"};
// はスカラの初期化と同様に2つ目の要素を飛ばせばよさそうに見えるが、 gcc では コンパイルエラーとなる。
// gcc では char str[] = "abc", "def"; のようにパースされているのかもしれないが、よく分からない。
// clang では3行上の例は valid な文法としてコンパイル可能。
// 
/// 生成規則:
/// array-initializer = (initializer ("," initializer)* ","? "}"
///	C99 以降の designator は現段階ではサポートしない
fn array_initializer(token_ptr: &mut TokenRef, typ: TypeCell) -> Initializer {
	let mut init = Initializer::default();
	let elem_typ = if let Ok(_typ) = typ.make_deref() { _typ } else { typ.clone() };
	loop {
		if is(token_ptr, "{") || elem_typ.is_non_array() {
			init.push_element(initializer(token_ptr, elem_typ.clone()));
		} else {
			// この深さではまだ配列が来るべきであるにも関わらず、初期化文のネストが浅かった場合の処理
			let (base_typ, elem_flatten_size) =
			if is_kind(token_ptr, Tokenkind::StringTk) && elem_typ.get_base_cell().typ != Type::Ptr {
				// 文字列リテラルかつ最小要素の型がポインタでない場合は、ベースの型を1次元配列とみなして読む(型チェックは initializer() で行うためここではスルー)
				let _typ = elem_typ.get_last_level_array().unwrap();
				let _flatten_size = elem_typ.flatten_size()/_typ.array_size.unwrap();
				(_typ, _flatten_size)
			} else {
				(elem_typ.get_base_cell(), elem_typ.flatten_size())
			};
			for _ in 0..elem_flatten_size {
				let elem = initializer(token_ptr, base_typ.clone());
				// base_typ が Array (つまり上記で文字リテラルを読んでいてかつポインタ型配列でない)の場合には、要素数カウントを正しく行うため、elem.elements を init.elements に append する
				if base_typ.is_array() {
					init.append_elements(&elem);
				} else {
					init.push_element(elem);
				}
				let _ = consume(token_ptr, ",");
				if is(token_ptr, "}") { break; }
			}
		}
		let _ = consume(token_ptr, ",");
		if consume(token_ptr, "}") { break; }
	}

	// 配列の Initializer の node は最初の要素を指すことにする
	let first_elem = init.elements[0].borrow().clone();
	init.insert(typ, Rc::clone(first_elem.node.as_ref().unwrap()));
	init
}

/// オフセットで直接代入したい場合の LvarNd
#[inline]
fn direct_offset_lvar(offset: usize, typ: TypeCell) -> NodeRef {
	Rc::new(RefCell::new(Node{ kind: Nodekind::LvarNd, typ: Some(typ), offset: Some(offset), is_local: true, ..Default::default() }))
}

/// initializer が存在する要素に対応する部分のみノードを作る(この時、flex であっても先に要素数は確定しており typ.array_size を利用して処理できる)
/// int x[2] = {1, 2}; のようなパターンは int x[2]; x[0] = 1, x[1] = 2; のように展開する
/// ただし、それぞれの要素アクセスのためにわざわざポインタ計算を生成せず、単に各要素が格納されるべき位置に対応するベースポインタからオフセットを持つローカル変数であるとみなす
fn make_lvar_init(init: Initializer, typ: TypeCell, offset: usize, is_scalar: bool, token: TokenRef) -> NodeRef {
	if typ.is_array() {
		let total_bytes = typ.bytes();
		let elem_typ = typ.make_deref().unwrap();
		let elem_bytes = elem_typ.bytes();
		let elem_flatten_size = elem_typ.flatten_size();
		let base_typ = typ.get_base_cell();
		let base_bytes = base_typ.bytes();
		let mut node_ptr = nop();

		let mut ix = 0;
		let mut finished_bytes = 0;
		while finished_bytes < total_bytes && ix < init.elements.len() {
			let elem = Rc::clone(&init.elements[ix]);
			if elem.borrow().is_element() {
				// flatten して読む
				for _ in 0..elem_flatten_size {
					let _expr = init.elements[ix].borrow().node.clone().unwrap();
					let _val = _expr.borrow().val.clone();
					// ゼロクリアが必ず入るため、0 を代入するだけのノードは無視する
					if _val.is_none() || _val.unwrap() != 0 {
						let _assign = assign_op(
							Nodekind::AssignNd,
							direct_offset_lvar(offset - finished_bytes, base_typ.clone()),
							_expr,
							Rc::clone(&token)
						);
						node_ptr = new_binary(Nodekind::CommaNd, node_ptr, _assign, Rc::clone(&token));
					}
					ix += 1;
					finished_bytes += base_bytes;
					if finished_bytes >= total_bytes || ix >= init.elements.len() { break; }
				}
			} else {
				node_ptr = new_binary(
					Nodekind::CommaNd,
					node_ptr,
					make_lvar_init(elem.borrow().clone(), elem_typ.clone(), offset - finished_bytes, false, Rc::clone(&token)),
					Rc::clone(&token)
				);
				ix += 1;
				finished_bytes += elem_bytes;
			}
		}

		node_ptr
		
	} else {
		let node_ptr = Rc::clone(init.node.as_ref().unwrap());
		let val = node_ptr.borrow().val.clone();
		// スカラ値の初期化時は0でもちゃんと初期化を行う AssignNd を生成する必要がある
		if val.is_none() || val.unwrap() != 0 || is_scalar {
			assign_op(
				Nodekind::AssignNd,
				direct_offset_lvar(offset, typ.clone()),
				node_ptr,
				token
			)
		} else {
			nop()
		}
	}
}

/// 生成規則:
/// stmt = expr? ";"
///		| declaration
///		| "{" stmt* "}" 
///		| "if" "(" expr ")" stmt ("else" stmt)?
///		| "while" "(" expr ")" stmt
///		| "for" "(" expr? ";" expr? ";" expr? ")" stmt
///		| "return" expr? ";"
fn stmt(token_ptr: &mut TokenRef) -> NodeRef {
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, ";") {
		tmp_num!(0)
	} else if is_type(token_ptr) {
		declaration(token_ptr)
	} else if consume(token_ptr, "{") {
		enter_scope();
		let mut children: Vec<Option<NodeRef>> = vec![];
		loop {
			if !consume(token_ptr, "}") {
				if at_eof(token_ptr) { exit_eprintln!("\'{{\'にマッチする\'}}\'が見つかりません。"); }
				let _stmt = stmt(token_ptr);
				confirm_type(&_stmt);
				children.push(Some(_stmt));
			} else {
				break;
			}
		}
		leave_scope();

		new_block(children)

	} else if consume(token_ptr, "if") {
		expect(token_ptr, "(");
		let enter= Some(expr(token_ptr));
		expect(token_ptr, ")");
		let branch = Some(stmt(token_ptr));
		let els = if consume(token_ptr, "else") { Some(stmt(token_ptr)) } else {None};

		new_ctrl(Nodekind::IfNd, None, enter, None, branch, els)

	} else if consume(token_ptr, "while") {
		expect(token_ptr, "(");
		let enter= Some(expr(token_ptr));
		expect(token_ptr, ")");
		let branch = Some(stmt(token_ptr)) ;

		new_ctrl(Nodekind::WhileNd, None, enter, None, branch, None)

	} else if consume(token_ptr, "for") {
		expect(token_ptr, "(");
		enter_scope();
		// consumeできた場合exprが何も書かれていないことに注意
		let init: Option<NodeRef> =
		if is_type(token_ptr) {
			Some(declaration(token_ptr))
		} else if consume(token_ptr, ";") {None} else {
			let _init = Some(expr(token_ptr));
			expect(token_ptr, ";");
			_init
		};

		let enter: Option<NodeRef> =
		if consume(token_ptr, ";") {None} else {
			let _enter = Some(expr(token_ptr));
			expect(token_ptr, ";");
			_enter
		};

		let routine: Option<NodeRef> = 
		if consume(token_ptr, ")") {None} else {
			let _routine = Some(expr(token_ptr));
			expect(token_ptr, ")");
			_routine
		};

		let branch: Option<NodeRef> = Some(stmt(token_ptr));
		leave_scope();

		new_ctrl(Nodekind::ForNd, init, enter, routine, branch, None)

	} else if consume_kind(token_ptr, Tokenkind::ReturnTk) {
		// exprなしのパターン: 実質NumNd 0があるのと同じと捉えれば良い
		let left: NodeRef =  
		if consume(token_ptr, ";") {
			tmp_num!(0)
		} else {
			let _left: NodeRef = expr(token_ptr);
			expect(token_ptr, ";");
			_left
		};

		new_unary(Nodekind::ReturnNd, left, token)

	} else {
		let node_ptr: NodeRef = expr(token_ptr);
		expect(token_ptr, ";");
		node_ptr
	}
}

/// 生成規則:
/// expr = assign ("," expr)? 
pub fn expr(token_ptr: &mut TokenRef) -> NodeRef {
	let node_ptr: NodeRef = assign(token_ptr);
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, ",") {
		new_binary(Nodekind::CommaNd, node_ptr, expr(token_ptr), token)
	} else {
		node_ptr
	}
}

/// 生成規則:
/// assign = logor (assign-op assign)?
/// assign-op = "="
///			| "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "^=" | "|="
///			| "<<=" | ">>="
/// 禁止代入(例えば x + y = 10; や x & y = 10; など)は generator 側で弾く
fn assign(token_ptr: &mut TokenRef) -> NodeRef {
	let node_ptr: NodeRef = logor(token_ptr);
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, "=") {
		assign_op(Nodekind::AssignNd, node_ptr,  assign(token_ptr), token)	
	} else if consume(token_ptr, "+=") {
		assign_op(Nodekind::AddNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "-=") {
		assign_op(Nodekind::SubNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "*=") {
		assign_op(Nodekind::MulNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "/=") {
		assign_op(Nodekind::DivNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "%=") {
		assign_op(Nodekind::ModNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "&=") {
		assign_op(Nodekind::BitAndNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "^=") {
		assign_op(Nodekind::BitXorNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "|=") {
		assign_op(Nodekind::BitOrNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, "<<=") {
		assign_op(Nodekind::LShiftNd, node_ptr, assign(token_ptr), token)
	} else if consume(token_ptr, ">>=") {
		assign_op(Nodekind::RShiftNd, node_ptr, assign(token_ptr), token)
	} else {
		node_ptr
	} 
}

/// a += b; -->  tmp = &a, *tmp = *tmp + b; と読み替える
fn assign_op(kind: Nodekind, left: NodeRef, right: NodeRef, token: TokenRef) -> NodeRef {
	// 左右の型を確定させておく
	confirm_type(&left);
	confirm_type(&right);

	// この式全体の評価値は left (a += b の a) の型とする
	let assign_ = 
	if kind == Nodekind::AssignNd {
		new_binary(Nodekind::AssignNd, left,  right, token)
	} else {
		// tmp として通常は認められない無名の変数を使うことで重複を避ける
		let typ = left.borrow().typ.clone().unwrap();
		let tmp_lvar = tmp_lvar!();
		let _ = tmp_lvar.borrow_mut().typ.insert(typ.make_ptr_to());
		let tmp_deref = tmp_unary!(Nodekind::DerefNd, tmp_lvar);

		let expr_left = tmp_binary!(
			Nodekind::AssignNd,
			tmp_lvar,
			tmp_unary!(Nodekind::AddrNd, left)
		);

		let token2 = Rc::clone(&token);
		let tmp_deref2 = Rc::clone(&tmp_deref);
		let op = match kind {
			Nodekind::AddNd => { new_add(tmp_deref2, right, token2) }
			Nodekind::SubNd => { new_sub(tmp_deref2, right, token2) }
			_ => { new_binary(kind, tmp_deref2, right, token2) }
		};

		let expr_right = tmp_binary!(
			Nodekind::AssignNd,
			tmp_deref,
			op
		);

		confirm_type(&expr_left);
		confirm_type(&expr_right);
		new_binary(Nodekind::CommaNd, expr_left, expr_right, token)
	};
	confirm_type(&assign_);
	assign_
}

/// 生成規則:
/// logor = logand ("||" logand)*
fn logor(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = logand(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, "||") { break; }
		node_ptr = new_binary(Nodekind::LogOrNd, node_ptr, logand(token_ptr), token);
	}
	node_ptr
}

/// 生成規則:
/// logand = bitor ("&&" bitor)*
fn logand(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = bitor(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, "&&") { break; }
		node_ptr = new_binary(Nodekind::LogAndNd, node_ptr, bitor(token_ptr), token);
	}
	node_ptr
}

/// 生成規則:
/// bitor = bitxor ("|" bitxor)*
fn bitor(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = bitxor(token_ptr);
	loop{
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, "|") { break; }
		node_ptr = new_binary(Nodekind::BitOrNd, node_ptr, bitxor(token_ptr), token);
	}
	node_ptr
}

/// 生成規則:
/// bitxor = bitand ("^" bitand)*
fn bitxor(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = bitand(token_ptr);
	loop{
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, "^") { break; }
		node_ptr = new_binary(Nodekind::BitXorNd, node_ptr, bitand(token_ptr), token);
	}
	node_ptr
}

/// 生成規則:
/// bitand = equality ("&" equality)*
fn bitand(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = equality(token_ptr);
	loop{
		let token = Rc::clone(token_ptr);
		if !consume(token_ptr, "&") { break; }
		node_ptr = new_binary(Nodekind::BitAndNd, node_ptr, equality(token_ptr), token);
	}
	node_ptr
}

/// 生成規則:
/// equality = relational ("==" relational | "!=" relational)?
fn equality(token_ptr: &mut TokenRef) -> NodeRef {
	let node_ptr: NodeRef = relational(token_ptr);
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, "==") {
		new_binary(Nodekind::EqNd, node_ptr, relational(token_ptr), token)
	} else if consume(token_ptr, "!=") {
		new_binary(Nodekind::NEqNd, node_ptr, relational(token_ptr), token)
	} else {
		node_ptr
	}
}

/// 生成規則:
/// relational = shift ("<" shift | "<=" shift | ">" shift | ">=" shift)*
fn relational(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = shift(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if consume(token_ptr, "<") {
			node_ptr = new_binary(Nodekind::LThanNd, node_ptr, shift(token_ptr), token);
		} else if consume(token_ptr, "<=") {
			node_ptr = new_binary(Nodekind::LEqNd, node_ptr, shift(token_ptr), token);
		} else if consume(token_ptr, ">") {
			node_ptr = new_binary(Nodekind::LThanNd, shift(token_ptr), node_ptr, token);
		} else if consume(token_ptr, ">=") {
			node_ptr = new_binary(Nodekind::LEqNd, shift(token_ptr), node_ptr, token);
		} else{
			break;
		}
	}
	node_ptr
}

/// 生成規則:
/// shift = add ("<<" add | ">>" add)*
fn shift(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = add(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if consume(token_ptr, "<<") {
			node_ptr = new_binary(Nodekind::LShiftNd, node_ptr, add(token_ptr), token);
		} else if consume(token_ptr, ">>") {
			node_ptr = new_binary(Nodekind::RShiftNd, node_ptr, add(token_ptr), token);
		} else {
			break;
		}
	}
	node_ptr
}

fn new_add(mut left: NodeRef, mut right: NodeRef, token: TokenRef) -> NodeRef {
	confirm_type(&left);
	confirm_type(&right);

	// それぞれ配列の場合でも true になるが、それで良い
	let left_is_ptr= left.borrow().typ.as_ref().unwrap().ptr_end.is_some();
	let right_is_ptr = right.borrow().typ.as_ref().unwrap().ptr_end.is_some();
	if left_is_ptr && right_is_ptr { error_with_token!("ポインタ演算は整数型との加算か、ポインタ同士の引き算のみ可能です。", &token.borrow()); }

	if !left_is_ptr && !right_is_ptr {
		new_binary(Nodekind::AddNd, left, right, token)
	} else {
		// num + ptr の場合には ptr + num として扱うべく左右を入れ替える
		if !left_is_ptr {
			let tmp = left;
			left = right;
			right = tmp;
		}

		// 配列の場合、サイズを考慮する必要があることに注意
		let ptr_cell = left.borrow().typ.clone().unwrap();
		let bytes = ptr_cell.ptr_to.as_ref().unwrap().borrow().bytes() as i32;
		let pointer_offset = tmp_binary!(Nodekind::MulNd, tmp_num!(bytes), right);
		let add_ = new_binary(Nodekind::AddNd, left, pointer_offset, token);
		confirm_type(&add_);
		let _ = add_.borrow_mut().typ.insert(ptr_cell);
		add_
	}
}

fn new_sub(left: NodeRef, right: NodeRef, token: TokenRef) -> NodeRef {
	confirm_type(&left);
	confirm_type(&right);
	let left_typ = left.borrow().typ.clone().unwrap();
	let right_typ = right.borrow().typ.clone().unwrap();

	// それぞれ配列の場合でも true になるが、それで良い
	let left_is_ptr= left_typ.ptr_end.is_some();
	let right_is_ptr = right_typ.ptr_end.is_some();

	let (sub_, type_cell) = 
	if !left_is_ptr && !right_is_ptr { 
		return new_binary(Nodekind::SubNd, left, right, token);
	} else if left_is_ptr && right_is_ptr {
		// ptr - ptr はそれが変数何個分のオフセットに相当するかを計算する
		if left_typ != right_typ { error_with_token!("違う型へのポインタ同士の演算はサポートされません。: \"{}\", \"{}\"", &token.borrow(), left_typ, right_typ);}

		let bytes = left_typ.ptr_to.as_ref().unwrap().borrow().bytes() as i32;
		let pointer_offset = tmp_binary!(Nodekind::SubNd, left, right);
		confirm_type(&pointer_offset);
		(new_binary(Nodekind::DivNd, pointer_offset, tmp_num!(bytes), token), TypeCell::new(Type::Int))

	} else {
		// num - ptr は invalid
		if !left_is_ptr { error_with_token!("整数型の値からポインタを引くことはできません。", &token.borrow()); }

		let bytes = left_typ.ptr_to.as_ref().unwrap().borrow().bytes() as i32;
		let pointer_offset = tmp_binary!(Nodekind::MulNd, tmp_num!(bytes), right);
		confirm_type(&pointer_offset);
		(new_binary(Nodekind::SubNd, left, pointer_offset, token), left_typ)
	};
	let _ = sub_.borrow_mut().typ.insert(type_cell);
	sub_
}

/// 生成規則:
/// add = mul ("+" mul | "-" mul)*
fn add(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = mul(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if consume(token_ptr, "+") {
			node_ptr = new_add( node_ptr, mul(token_ptr), token);
		} else if consume(token_ptr, "-") {
			node_ptr = new_sub(node_ptr, mul(token_ptr), token);
		} else {
			break;
		}
	}
	node_ptr
}

/// 生成規則:
/// mul = unary ("*" unary | "/" unary | "%" unary)*
fn mul(token_ptr: &mut TokenRef) -> NodeRef {
	let mut node_ptr: NodeRef = unary(token_ptr);
	loop {
		let token = Rc::clone(token_ptr);
		if consume(token_ptr, "*") {
			node_ptr = new_binary(Nodekind::MulNd, node_ptr, unary(token_ptr), token);
		} else if consume(token_ptr, "/") {
			node_ptr = new_binary(Nodekind::DivNd, node_ptr, unary(token_ptr), token);
		} else if consume(token_ptr, "%") {
			node_ptr = new_binary(Nodekind::ModNd, node_ptr, unary(token_ptr), token);
		} else {
			break;
		}
	}
	node_ptr
}

/// !+x; や ~-y; は valid
/// unary = tailed 
///		| ("sizeof") ( "(" (type | expr) ")" | unary)
///		| ("~" | "!") unary
///		| ("*" | "&") unary 
///		| ("+" | "-") unary
///		| ("++" | "--") unary 
// TODO: *+x; *-y; みたいな構文を禁止したい
fn unary(token_ptr: &mut TokenRef) -> NodeRef {
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, "sizeof") {
		// 型名を使用する場合は括弧が必要なので sizeof type になっていないか先にチェックする
		let ptr_ = Rc::clone(token_ptr);
		if let Some(typ) = consume_type(token_ptr) {
			error_with_token!("型名を使用した sizeof 演算子の使用では、 \"(\" と \")\" で囲う必要があります。 -> \"({})\"", &ptr_.borrow(), typ);
		}
		let typ: TypeCell = if consume(token_ptr, "(") {
			let typ_: TypeCell =  if let Some(t) = consume_type(token_ptr) {
				t
			} else {
				let exp = expr(token_ptr);
				confirm_type(&exp);
				let _typ = exp.borrow().typ.clone().unwrap();
				_typ
			};
			expect(token_ptr, ")");
			typ_
		} else {
			let una = unary(token_ptr);
			confirm_type(&una);
			let _typ = una.borrow().typ.clone().unwrap();
			_typ
		};
		new_num(typ.bytes() as i32, token)
	} else if consume(token_ptr, "~") {
		new_unary(Nodekind::BitNotNd, unary(token_ptr), token)
	} else if consume(token_ptr, "!") {
		new_unary(Nodekind::LogNotNd, unary(token_ptr), token)
	} else if consume(token_ptr, "*") {
		let node_ptr = unary(token_ptr);
		confirm_type(&node_ptr);
		new_unary(Nodekind::DerefNd, node_ptr, token)
	} else if consume(token_ptr, "&") {
		let node_ptr = unary(token_ptr);
		confirm_type(&node_ptr);
		new_unary(Nodekind::AddrNd, node_ptr, token)
	} else if consume(token_ptr, "+") {
		// 単項演算子のプラスは0に足す形にする。こうすることで &+var のような表現を generator 側で弾ける
		new_binary(Nodekind::AddNd, tmp_num!(0), primary(token_ptr), token)
	} else if consume(token_ptr, "-") {
		// 単項演算のマイナスは0から引く形にする。
		new_binary(Nodekind::SubNd, tmp_num!(0), primary(token_ptr), token)
	} else if consume(token_ptr, "++") {
		assign_op(Nodekind::AddNd, unary(token_ptr), tmp_num!(1), token)
	} else if consume(token_ptr, "--") {
		assign_op(Nodekind::SubNd, unary(token_ptr), tmp_num!(1), token)
	} else {
		tailed(token_ptr)
	}
}

/// 生成規則:
/// tailed = primary (primary-tail)?
/// primary-tail = "++" | "--"
fn tailed(token_ptr: &mut TokenRef) -> NodeRef {
	let node_ptr: NodeRef = primary(token_ptr);
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, "++") {
		inc_dec(node_ptr, true, false, token)
	} else if consume(token_ptr, "--") {
		inc_dec(node_ptr, false, false, token)
	} else {
		node_ptr
	}
}

/// ++a; -> a+=1; および a++; -> (a+=1)-1; と読み替える
fn inc_dec(node: NodeRef, is_inc: bool, is_prefix: bool, token: TokenRef) -> NodeRef {
	let kind = if is_inc { Nodekind::AddNd } else { Nodekind::SubNd };
	confirm_type(&node);
	let _assign = assign_op(kind, node, tmp_num!(1), Rc::clone(&token));
	if is_prefix {
		// ++i は (i+=1) として読み替えると良い
		_assign
	} else {
		// i++ は (i+=1)-1 として読み替えると良い
		if is_inc {
			new_sub(_assign, tmp_num!(1), token)
		} else {
			new_add(_assign, tmp_num!(1), token)
		}
	}
}

/// 生成規則:
/// params = assign ("," assign)* | null
fn params(token_ptr: &mut TokenRef) -> Vec<Option<NodeRef>> {
	let mut args: Vec<Option<NodeRef>> = vec![];
	if !consume(token_ptr, ")") {
		let arg = assign(token_ptr);
		confirm_type(&arg);
		args.push(Some(arg));

		loop {
			if !consume(token_ptr, ",") {
				expect(token_ptr,")"); // 括弧が閉じないような書き方になっているとここで止まるため、if at_eof ~ のようなチェックは不要
				break;
			}
			let arg = assign(token_ptr);
			confirm_type(&arg);
			args.push(Some(arg));
		}
	}
	args
}

/// 生成規則: 
/// primary = num
///			| string-literal
///			| ident ( "(" params ")" | "[" expr "]")?
///			| "(" expr ")"
fn primary(token_ptr: &mut TokenRef) -> NodeRef {
	let token = Rc::clone(token_ptr);
	if consume(token_ptr, "(") {
		let node_ptr: NodeRef = expr(token_ptr);
		expect(token_ptr, ")");
		node_ptr
	} else if let Some(name) = consume_ident(token_ptr) {
		if consume(token_ptr, "(") {
			let func_typ: TypeCell;
			let args:Vec<Option<NodeRef>> = params(token_ptr);
			// 本来、宣言されているかを contains_key で確認したいが、今は外部の C ソースとリンクさせているため、このコンパイラの処理でパースした関数に対してのみ引数の数チェックをするにとどめる。
			let glb_access = GLOBALS.try_lock().unwrap();
			if glb_access.contains_key(&name) {
				let glob = glb_access.get(&name).unwrap();
				func_typ =
				if let Some(_typ) = glob.func_typ.clone() { _typ }
				else { error_with_token!("型\"{}\"は関数として扱えません。", &*token.borrow(), glob.typ.clone().unwrap()); };

				// 現在利用できる型は一応全て エラーレベルで compatible (ただしまともなコンパイラは warning を出す) なので、引数の数があっていれば良いものとする
				let argc = func_typ.arg_typs.as_ref().unwrap().len();
				if args.len() != argc { error_with_token!("\"{}\" の引数は{}個で宣言されていますが、{}個が渡されました。", &*token.borrow(), name, argc, args.len()); }
				new_funcall(name, func_typ, args, token)
			} else {
				// 外部ソースの関数の戻り値の型をコンパイル時に得ることはできないため、int で固定とする
				// また、引数の型は正しいとして args のものをコピーする
				let mut arg_typs = vec![];
				for arg in &args {
					arg_typs.push(Rc::new(RefCell::new(arg.as_ref().unwrap().borrow().typ.clone().unwrap())));
				}
				func_typ = TypeCell::new(Type::Int).make_func(arg_typs);
				new_funcall(name, func_typ, args, token)
			}
		} else {
			// グローバル変数については、外部ソースとのリンクは禁止として、LOCALS, GLOBALS に当たらなければエラーになるようにする
			let mut typ: TypeCell = TypeCell::default();
			let mut is_local = false;
			let mut level = 0;
			for (l, scope) in LOCALS.try_lock().unwrap().iter_mut().enumerate().rev() {
				let _ = scope.entry(name.clone()).and_modify(|(_, t)| {
					typ = t.clone();
					level = l;
					is_local = true;
				});
				if is_local { break; }
			}

			if !is_local {
				typ = GLOBALS.try_lock().unwrap().entry(name.clone())
					.or_insert_with(|| {
						error_with_token!("定義されていない変数です。", &*token.borrow());
					})
					.typ.clone().unwrap();
			}

			let mut node_ptr = new_lvar(name, Rc::clone(&token), typ, is_local, level);
			while consume(token_ptr, "[") {
				let index_token = Rc::clone(token_ptr);
				let index = expr(token_ptr);
				node_ptr = new_unary(Nodekind::DerefNd, new_add(node_ptr, index, index_token), Rc::clone(&token));
				expect(token_ptr,"]");
			}
			node_ptr
		}
	} else if let Some(literal) = consume_literal(token_ptr) {
		let size = literal.len() + 1;
		let name = store_literal(literal);
		new_lvar(name, token, TypeCell::new(Type::Char).make_array_of(size), false, 0)
	} else {
		new_num(expect_number(token_ptr), token)
	}
}

#[cfg(test)]
pub mod tests {
	use crate::tokenizer::tokenize;
	use crate::globals::{SRC, FILE_NAMES};
	use super::*;
	
	static REP: usize = 40;

	fn test_init(src: &str) {
		let mut src_: Vec<String> = src.split("\n").map(|s| s.to_string()+"\n").collect();
		FILE_NAMES.try_lock().unwrap().push("test".to_string());
		let mut code = vec!["".to_string()];
		code.append(&mut src_);
		SRC.try_lock().unwrap().push(code);
	}

	fn search_tree(tree: &NodeRef) {
		let node: &Node = &*(*tree).borrow();
		println!("{}", node);

		if node.left.is_some() {search_tree(node.left.as_ref().unwrap());}
		if node.right.is_some() {search_tree(node.right.as_ref().unwrap());}
		if node.init.is_some() {search_tree(node.init.as_ref().unwrap());}
		if node.enter.is_some() {search_tree(node.enter.as_ref().unwrap());}
		if node.routine.is_some() {search_tree(node.routine.as_ref().unwrap());}
		if node.branch.is_some() {search_tree(node.branch.as_ref().unwrap());}
		if node.els.is_some() {search_tree(node.els.as_ref().unwrap());}
		for child in &node.children {
			if child.is_some() {search_tree(child.as_ref().unwrap());}
		}
		for arg in &node.args {
			if arg.is_some() {search_tree(arg.as_ref().unwrap());}
		}
		if node.stmts.is_some() {
			for stmt_ in node.stmts.as_ref().unwrap() {
				search_tree(stmt_);
			}
		}
	}

	pub fn parse_stmts(token_ptr: &mut TokenRef) -> Vec<NodeRef> {
		let mut stmts :Vec<NodeRef> = Vec::new();
		enter_scope();
		while !at_eof(token_ptr) {
			let stmt_ = stmt(token_ptr);
			confirm_type(&stmt_);
			stmts.push(stmt_);
		}
		leave_scope();
		stmts
	}

	#[test]
	fn basic_calc() {
		let src: &str = "
			int x, y, z;
			x = 1 + 2 / 1;
			y = 200 % (3 + 1);
			z = 30 % 3 + 2 * 4;
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn shift() {
		let src: &str = "
			int x;
			x = 10 << 2 + 3 % 2 >> 3;
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn bitops() {
		let src: &str = "
			int x, z;
			int* y;
			2 + (3 + 5) * 6;
			1 ^ 2 | 2 != 3 / 2;
			1 + -1 ^ 2;
			3 ^ 2 & 1 | 2 & 9;
			x = 10;
			y = &x;
			3 ^ 2 & *y | 2 & &x;
			z = ~x;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn logops() {
		let src: &str = "
			1 && 2 || 3 && 4;
			1 && 2 ^ 3 || 4 && 5 || 6;
			!2;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn inc_dec() {
		let src: &str = "
			int i;
			i = 0;
			++i;
			--i;
			i++;
			i--;
			int *p;
			++*p;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn for_() {
		let src: &str = "
			int sum, i;
			sum = 10;
			sum = sum + i;
			for (i = 1 ; i < 10; i = i + 1) sum = sum +i;
			return sum;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn while_() {
		let src: &str = "
			int sum;
			sum = 10;
			while(sum > 0) sum = sum - 1;
			return sum;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn if_() {
		let src: &str = "
			int i;
			i = 10;
			if (i == 10) i = i / 5;
			if (i == 2) i = i + 5; else i = i / 5;
			return i;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn ctrls() {
		let src: &str = "
			int i, sum;
			sum = 0;
			i = 10;
			if (i == 10) while(i < 0) for(;;) sum = sum + 1;
			return sum;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn block() {
		let src: &str = "
			for( i = 10; ; ) {i = i + 1;}
			{}
			{i = i + 1; 10;}
			return 10;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn block2() {
		let src: &str = "
			int i, x;
			while(i < 10) {i = i + 1; i = i * 2;}
			x = 10;
			if ( x == 10 ){
				x = x + 200;
				x = x / 20;
			} else {
				x = x - 20;
				;
			}
			{{}}
			{i = i + 1; 10;}
			return 200;
			return;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn func() {
		let src: &str = "
			int i, j;
			call_fprint();
			i = getOne();
			j = getTwo();
			return i + j;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn func2() {
		let src: &str = "
			int i, j, k;
			call_fprint();
			i = get(1);
			j = get(2, 3, 4);
			k = get(i+j, (i=3), k);
			return i + j;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn addr_deref() {
		let src: &str = "
			int x, y, z;
			x = 3;
			y = 5;
			z = &y + 8;
			return *z;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn addr_deref2() {
		let src: &str = "
			int x;
			int *y;
			int **z;
			x = 3;
			y = &x;
			z = &y;
			return *&**z;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn comma() {
		let src: &str = "
			int x, y, z;
			x = 3, y = 4, z = 10;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn assign_op() {
		let src: &str = "
			int x;
			x = 10;
			x += 1;
			x <<= 1;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn sizeof() {
		let src: &str = "
			int x, y, z;
			x = 0; y = 0; z = 0;
			int *p; p = &x;
			int *pp; pp = &p;

			sizeof(int);
			sizeof(int **);
			sizeof(0);
			sizeof(x);
			sizeof x;
			sizeof ++x;
			sizeof ++p;
			sizeof(x+y);
			sizeof x + y * z;
			sizeof(x && x);
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn array() {
		let src: &str = "
			int x[20];
			int y[10][20];
			int *p[10][20][30];
			int *q;
			int z;
			sizeof(*y);
			sizeof(x);
			x - q;
			x + 10;
			y - &q;
			**p - y;
			&p - z;
			****p;
			*****&p;
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = parse_stmts(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("stmt{} {}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn declare() {
		let src: &str = "
			int func(int x, int y) {
				return x + y;
			}
			int calc(int a, int b, int c, int d, int e, int f) {
				return a*b + c - d + e/f;
			}
			int main() {
				int i, sum;
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
				return func(i, sum);
			}
		";
		test_init(src);
		
		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn no_return() {
		let src: &str = "
			int func(int x, int y) {
				return x + y;
			}
			int main() {
				int i, sum, x, y, z;
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
				func(x=1, (y=1, z=1));
			}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn array_access() {
		let src: &str = "
		int main() {
			int X[10][10][10];
			X[0][0][0] = 10;
			return ***X;
		}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn gvar() {
		let src: &str = "
		int x;
		int X[0][0][0];
		int func(int x, int y) {
			return x + y;
		}
		int main() {
			X;
			int X[10][10][10];
			X[0][0][0] = 10;
			func(1, 3) + 1;
			return ***X;
		}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn literal() {
		let src: &str = "
		int main() {
			char *c = \"aaaa\";
			\"bbbb\";
			*c = 60;
			return 0;
		}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn lvar_init() {
		let src: &str = "
			int main() {
				int x = {4, 5};
				int X[4][2][1] = {1, {2, 3}, x, 5, {6}, 7, 8, 9, };
				char str[][2][2] = {{{{\"str\", }}}};
				char str2[][2][3] = {\"s\", \"abcd\", \"pqrs\"};
				char str3[] = {\"str\", };
				// int str4[][2] = {\"str\"}; // invalid
			}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		} 
	}

	#[test]
	fn gvar_init() {
		let src: &str = "
			int x = {4+10, 5};
			int y = 10;
			int *p = &x - &x + &y; // valid
			int *q = &y + 10; // valid
			// int *r = &x - &y; // invalid
			int z[][2][10] = {{1, 4, 5, {10, x}}, 2, 3, 4, 2}; // valid
			// int w[][2][10] = {{1, 4, 5, {10, xx}}, 2, 3, 4, 2}; // invalid
			char c[10][1000] = {\"abcd\", \"str\", {\'c\'}}, d[] = \"compiler\";
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	#[test]
	fn scope() {
		let src: &str = "
		int main() {
			int a = {0};
			int i = 10;
			for (int i = 0; i < 10; i++) {
				int i = 10;
				a++;
			}
			print_helper(i);
			print_helper(a);

			return 0;
		}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}

	// wip() を「サポートしている構文を全て使用したテスト」と定めることにする
	#[test]
	fn wip() {
		let src: &str = "
		int fib(int);
		int MEMO[100];
		int X[10][20][30];
		char c[10];

		int main() {
			int i, x;
			int *p = &X[0][0][0];
			int **pp = &p;
			***X = 10;

			for(i=0; i < 100; i++) {
				MEMO[i] = 0;
			}
			
			X[0][3][2] = 99;
			print_helper(X[0][2][32]);
			print_helper(sizeof X);

			int X[10][10][10];
			print_helper(sizeof &X);
			print_helper(X);
			print_helper(X[1]);
			print_helper(&X+1);
			X[0][1][1] = 100;
			
			print_helper((x = 19, x = fib(*&(**pp))));
			print_helper(fib(50));

			char *str = \"This is test script\";
			int t = *str;
			print_helper(t);

			return x;
		}

		int fib(int N) {
			if (N <= 2) return 1;
			if (MEMO[N-1]) return MEMO[N-1];
			return MEMO[N-1] = fib(N-1) + fib(N-2);
		}
		";
		test_init(src);

		let mut token_ptr = tokenize(0);
		let node_heads = program(&mut token_ptr);
		let mut count: usize = 1;
		for node_ptr in node_heads {
			println!("declare{}{}", count, ">".to_string().repeat(REP));
			search_tree(&node_ptr);
			count += 1;
		}
	}
}