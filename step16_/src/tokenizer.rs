use crate::{
	error_with_token,
	token::{Token, Tokenkind, token_ptr_exceed},
	utils::{strtol, is_digit, error_at, CODES}
};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::rc::Rc;
use std::cell::RefCell;
use std::iter::FromIterator;

// 入力文字列のトークナイズ
pub fn tokenize(file_num: usize) -> Rc<RefCell<Token>> {
	// Rcを使って読み進める
	let mut token_ptr: Rc<RefCell<Token>> = Rc::new(RefCell::new(Token::new(Tokenkind::HeadTk,"", 0, 0, 0)));
	let mut token_head_ptr: Rc<RefCell<Token>> = token_ptr.clone(); // Rcなのでcloneしても中身は同じものを指す
	let code = &mut CODES.lock().unwrap()[file_num];

	for (line_num, string) in (&*code).iter().enumerate() {

		// StringをVec<char>としてlookat(インデックス)を進めることでトークナイズを行う(*char p; p++;みたいなことは気軽にできない)
		let len: usize = string.len();
		let mut lookat: usize = 0;
		let mut c: char;
		let string: Vec<char> = string.as_str().chars().collect::<Vec<char>>(); 

		while lookat < len {
			// 余白をまとめて飛ばす。streamを最後まで読んだならbreakする。
			match skipspace(&string, &mut lookat, len) {
				Ok(()) => {},
				Err(()) => {break;}
			}

			// 予約文字を判定
			if let Some(body) = is_reserved(&string, &mut lookat, len) {
				(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::ReservedTk, body, file_num, line_num, lookat))));
				token_ptr_exceed(&mut token_ptr);
				continue;
			}

			if is_return(&string, &mut lookat, len) {
				// トークン列にIdentTkとして追加する必要がある
				(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::ReturnTk, "", file_num, line_num, lookat))));
				token_ptr_exceed(&mut token_ptr);
				continue;
			}
			
			// 数字ならば、数字が終わるまでを読んでトークンを生成
			c = string[lookat];
			if is_digit(&c) {
				let num = strtol(&string, &mut lookat);
				(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::NumTk, num.to_string(), file_num, line_num, lookat))));
				token_ptr_exceed(&mut token_ptr);
				continue;
			}

			// 英字とアンダーバーを先頭とする文字を識別子としてサポートする
			if (c >= 'a' && c <= 'z') | (c >= 'A' && c <= 'Z') | (c == '_') {
				let name = read_lvar(&string, &mut lookat);

				// トークン列にIdentTkとして追加する必要がある
				(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::IdentTk, name, file_num, line_num, lookat))));
				token_ptr_exceed(&mut token_ptr);
				continue;
			}

			error_at("トークナイズできません", file_num, line_num, lookat);
		}
	}

	(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::EOFTk, "", 0, 0, 0))));
	token_ptr_exceed(&mut token_head_ptr);

	token_head_ptr
}

/* ------------------------------------------------- トークナイズ用関数 ------------------------------------------------- */

static TRI_OPS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec![
		"<<=", ">>=",
]));

static BI_OPS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec![
	"==", "!=", "<=", ">=", "&&", "||",
	"<<", ">>",
	"++", "--",
	"+=", "-=", "*=", "/=", "%=", "&=", "^=", "|=",
]));
static UNI_RESERVED: Lazy<Mutex<Vec<char>>> = Lazy::new(|| Mutex::new(vec![
	';', ',',
	'(', ')', '{', '}',
	'+', '-', '*', '/', '%', '&', '|', '^',
	'!', '~', 
	'=',
	'<', '>',
]));
static SPACES: Lazy<Mutex<Vec<char>>> = Lazy::new(|| Mutex::new(vec![
	' ', '\t', '\n'
]));

// 空白を飛ばして読み進める
fn skipspace(string: &Vec<char>, index: &mut usize, len: usize) -> Result<(), ()> {

	// 既にEOFだったならErrを即返す
	if *index >= len {
		return Err(());
	}

	// 空白でなくなるまで読み進める
	while SPACES.lock().unwrap().contains(&string[*index]) {
		*index += 1;
		if *index >= len {
			return Err(());
		}
	}

	Ok(())
}

// 識別子の一部として使用可能な文字であるかどうかを判別する
fn canbe_ident_part (c: &char) -> bool {
	return (*c >= 'a' && *c <= 'z') | (*c >= 'A' && *c <= 'Z') | (*c >= '0' && *c <= '9') | (*c == '_')
}

// 予約されたトークンの後に空白なしで連続して良い文字であるかどうかを判別する。
fn can_follow_reserved(string: &Vec<char>, index: usize) -> bool {
	if let Some(c) = string.get(index) {
		if UNI_RESERVED.lock().unwrap().contains(c) || SPACES.lock().unwrap().contains(c) {
			return true;
		}
		return false;
	}
	// indexがout of bounds(=前のトークンが文末にある)ならトークナイズを許して後でパーサにエラーを出させる方針
	true
}

// 予約されたトークンだった場合はSome(String)を返す
fn is_reserved(string: &Vec<char>, index: &mut usize, len: usize) -> Option<String> {

	// 先に複数文字の演算子かどうかチェックする(文字数の多い方から)
	let lim = *index+5;
	if lim <= len {
		let slice: String = String::from_iter(string[*index..lim].iter());
		if slice == "while" && can_follow_reserved(string,lim) {
			*index = lim;
			return Some(slice);
		}
	}

	// 先に複数文字の演算子かどうかチェックする(文字数の多い方から)
	let lim = *index + 4;
	if lim <= len {
		let slice: String = String::from_iter(string[*index..lim].iter());
		if slice == "else" && can_follow_reserved(string, lim) {
			*index = lim;
			return Some(slice);
		}
	}

	// 先に複数文字の演算子かどうかチェックする(文字数の多い方から)
	let lim = *index + 3;
	if lim <= len {
		let slice: String = String::from_iter(string[*index..lim].iter());
		if TRI_OPS.lock().unwrap().contains(&slice.as_str()) || slice == "for" && can_follow_reserved(string, lim) {
			*index = lim;
			return Some(slice);
		}
	}

	// 2文字演算子とif
	let lim = *index + 2;
	if lim <= len {
		let slice: String = String::from_iter(string[*index..(*index+2)].iter());
		if BI_OPS.lock().unwrap().contains(&slice.as_str()) || (slice == "if" && can_follow_reserved(string, lim))  {
			*index = lim;
			return Some(slice);
		}
	}

	// 単項演算子、括弧、代入演算子、文末のセミコロンを予約
	if *index < len {
		let c: char = string[*index];

		if UNI_RESERVED.lock().unwrap().contains(&c) {
			*index += 1;
			return Some(c.to_string());
		}
	}

	None
}

// return文を読む
fn is_return(string: &Vec<char>, index: &mut usize, len: usize) -> bool {
	// is_reservedと同じ要領でreturnを読み取る
	let lim = *index + 6;
	// stringの残りにそもそもreturnの入る余地がなければ即return(index out of range回避)
	if lim >= len {return false;}

	let slice: String = String::from_iter(string[*index..lim].iter());
	if slice == "return" && can_follow_reserved(string, lim){
		*index = lim;
		true
	} else {
		false
	}
}

// LVarに対応する文字列を抽出しつつ、indexを進める
fn read_lvar(string: &Vec<char>, index: &mut usize) -> String {
	let mut name = "".to_string();

	// 1文字ずつみて連結する
	while canbe_ident_part(&string[*index]) {
		name = format!("{}{}", name, string[*index]);
		*index += 1;
	}

	name
}

/* ------------------------------------------------- トークン処理用関数(parserからの呼び出しを含むためpubが必要) ------------------------------------------------- */

// 次のトークンが数字であることを期待して次のトークンを読む関数
pub fn expect_number(token_ptr: &mut Rc<RefCell<Token>>) -> i32 {
	if (**token_ptr).borrow().kind != Tokenkind::NumTk {
		error_with_token!("数字であるべき位置で数字以外の文字\"{}\"が発見されました。", &*token_ptr.borrow(),(**token_ptr).borrow().body.as_ref().unwrap());
	}
	let val = (**token_ptr).borrow().val.unwrap();
	token_ptr_exceed(token_ptr);
	
	val
}

// 次のトークンが識別子(変数など)であることを期待して次のトークンを読む関数
pub fn expect_ident(token_ptr: &mut Rc<RefCell<Token>>) -> String {
	if (**token_ptr).borrow().kind != Tokenkind::IdentTk {
		error_with_token!("識別子を期待した位置で\"{}\"が発見されました。", &*token_ptr.borrow(), (**token_ptr).borrow().body.as_ref().unwrap());
	}
	let body = (**token_ptr).borrow_mut().body.as_ref().unwrap().clone();
	token_ptr_exceed(token_ptr);
	
	body
}

//  予約済みトークンを期待し、(文字列で)指定して読む関数(失敗するとexitする)
pub fn expect(token_ptr: &mut Rc<RefCell<Token>>, op: &str) {
	if (**token_ptr).borrow().kind != Tokenkind::ReservedTk || (**token_ptr).borrow().body.as_ref().unwrap() != op {
		error_with_token!("\"{}\"を期待した位置で予約されていないトークン\"{}\"が発見されました。", &*token_ptr.borrow(), op, (**token_ptr).borrow().body.as_ref().unwrap());
	}
	token_ptr_exceed(token_ptr);
}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとfalseを返す)
pub fn consume(token_ptr: &mut Rc<RefCell<Token>>, op: &str) -> bool {
	if (**token_ptr).borrow().kind != Tokenkind::ReservedTk || (**token_ptr).borrow().body.as_ref().unwrap() != op {
		return false;
	}
	token_ptr_exceed(token_ptr);

	true
}

// 期待する次のトークンを(Tokenkindで)指定して読む関数(失敗するとfalseを返す)
pub fn consume_kind(token_ptr: &mut Rc<RefCell<Token>>, kind: Tokenkind) -> bool {
	if (**token_ptr).borrow().kind != kind {
		return false;
	}
	token_ptr_exceed(token_ptr);

	true
}

// 識別子であるかを判別する
pub fn is_ident(token_ptr: &Rc<RefCell<Token>>) -> bool {
	(**token_ptr).borrow().kind == Tokenkind::IdentTk
}

// EOFかどうかを判断する関数
pub fn at_eof(token_ptr: &Rc<RefCell<Token>>) -> bool{
	(**token_ptr).borrow().kind == Tokenkind::EOFTk
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lvar(){
		let src: Vec<String> ="
			local = -1;
			local_ = 2;
			local_1 = local_a = local;
			oops = 3;
			LOCAL = local * 30;
			local = (100 + 30 / 5 - 99) * (local > local);
			LOCAL + local*local + (LOCAL + local_)* local_1 + oops;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn return_(){
		let src: Vec<String> = "
			a = 1;
			b = a * 8;
			return8 = 9;
			_return = 0;
			return 11;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn ctrl(){
		let src: Vec<String> ="
			for( i = 10; ;  ) i = i + 1;
			x = 20;
			while(i == 0) x = x + 1;
			if_ = 10
			if(if_ >= 0) if_ - 100; else if_ * 100;
			return8 = 10;
			return return8;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn block(){
		let src: Vec<String> ="
			for( i = 10; ; ) {i = i + 1;}
			{}
			{i = i + 1;}
			return 10;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn addr_deref(){
		let src: Vec<String> ="
			x = 4;
			y = &x;
			z = &y;
			return *&**z;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn ops(){
		let src: Vec<String> ="
			x = 1;
			y = 0;
			z = 2;
			if( x || (y && z)) print_helper(x); else return z;
			w = x & y ^ z;
			p = !x;
			return ~z;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn ops2(){
		let src: Vec<String> ="
			x = 1;
			y = 0;
			z = 2;
			if( x || (y && z)) print_helper(x); else return z;
			w = x & y ^ z;
			p = !x;
			return ~z;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}

	#[test]
	fn ops3(){
		let src: Vec<String> ="
			x = 1;
			x += 1;
			x -= 1;
			x *= 1;
			x /= 1;
			x %= 1;
			x &= 1;
			x ^= 1;
			x |= 1;
			x <<= 1;
			x >>= 1;
			x++;
			x--;
			++x;
			--x;
		".split("\n").map(|s| s.into()).collect();
		{
			let code = &mut CODES.lock().unwrap()[0];
			for line in src { code.push(line); }
		}

		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		while (*token_ptr).borrow().kind != Tokenkind::EOFTk {
			println!("{}", (*token_ptr).borrow());
			token_ptr_exceed(&mut token_ptr);
		}
		assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
		println!("{}", (*token_ptr).borrow());
	}
}
