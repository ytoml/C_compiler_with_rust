use crate::{exit_eprintln};
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::{Display,  Formatter};
use std::fmt;
use std::iter::FromIterator;

#[derive(Debug, PartialEq)]
pub enum Tokenkind {
	DefaultTk, // Default用のkind
	HeadTk, // 先頭にのみ使用するkind
	IdentTk, // 識別子
	ReservedTk, // 記号
	NumTk, // 整数トークン
	ReturnTk, // リターン
	EOFTk, // 入力終わり
}

// Rc<RefCell<T>>により共有可能な参照(ポインタ風)を持たせる
pub struct Token {
	pub kind: Tokenkind,
	pub val: Option<i32>,  
	pub body: Option<String>,
	pub len: usize, // 1文字でないトークンもあるので、文字列の長さを保持しておく(非負)
	pub next: Option<Rc<RefCell<Token>>>, // Tokenは単純に単方向非循環LinkedListを構成することしかしないため、リークは起きないものと考える(循環の可能性があるなら、Weakを使うべき)
}

impl Default for Token {
	fn default() -> Token {
		Token {kind: Tokenkind::DefaultTk, val: None, body: None, len: 0, next: None}
	}

}

// 構造体にStringをうまく持たせるためのnewメソッド
impl Token {
	fn new(kind: Tokenkind, body: impl Into<String>) -> Token {
		let body: String = body.into();
		let len = body.chars().count(); // len()を使うとバイト数になってややこしくなるので注意
		match kind {
			Tokenkind::HeadTk => {
				Token {kind: kind, .. Default::default()}
			},
			Tokenkind::IdentTk => {
				Token {
					kind: kind, 
					body: Some(body),
					len: len,
					.. Default::default()
				}
			},
			Tokenkind::NumTk => {
				// NumTkと共に数字以外の値が渡されることはないものとして、unwrapで処理
				let val = body.parse::<i32>().unwrap();
				Token {
					kind: kind,
					val: Some(val),
					body: Some(body),
					len: len,
					next: None
				}
			},
			Tokenkind::ReservedTk => {
				Token {
					kind: kind,
					body: Some(body),
					len: len,
					.. Default::default()
				}
			},
			Tokenkind::ReturnTk => {
				Token {
					kind: kind, 
					body: Some("This is return Token.".to_string()),
					len: 6,
					.. Default::default()
				}
			}
			Tokenkind::EOFTk => {
				Token {
					kind: kind, 
					body: Some("This is EOF Token.".to_string()),
					.. Default::default()
				}
			},
			_ => Default::default() // DefaultTkの場合(想定されていない)
		}
	}
}


impl Display for Token {
	fn fmt(&self, f:&mut Formatter) -> fmt::Result {
		let mut s = format!("{}\n", "-".to_string().repeat(40));
		s = format!("{}Tokenkind : {:?}\n", s, self.kind);

		if let Some(e) = self.body.as_ref() {
			s = format!("{}body: {}\n", s, e);
		} else {
			s = format!("{}body: not exist\n", s);
		}

		if let Some(e) = self.val.as_ref() {
			s = format!("{}val: {}\n", s, e);
		} else {
			s = format!("{}val: not exist\n", s);
		}

		if let Some(e) = self.next.as_ref() {
			s = format!("{}next: exist(kind:{:?})\n", s, (**e).borrow().kind);
		} else {
			s = format!("{}next: not exist\n", s);
		}

		writeln!(f, "{}", s)
	}
}



// トークンのポインタを読み進める
pub fn token_ptr_exceed(token_ptr: &mut Rc<RefCell<Token>>) {
	let tmp_ptr;

	// nextがNoneならパニック
	match (**token_ptr).borrow().next.as_ref() {
		Some(ptr) => {
			tmp_ptr = ptr.clone();
		},
		None => {
			exit_eprintln!("次のポインタを読めません。(現在のポインタのkind:{:?})", (**token_ptr).borrow().kind);
		}
	}
	*token_ptr = tmp_ptr;
}


// 入力文字列のトークナイズ
pub fn tokenize(string: String) -> Rc<RefCell<Token>> {
	// Rcを使って読み進める
	let mut token_ptr: Rc<RefCell<Token>> = Rc::new(RefCell::new(Token::new(Tokenkind::HeadTk,"")));
	let mut token_head_ptr = token_ptr.clone(); // Rcなのでcloneしても中身は同じものを指す
	

	// StringをVec<char>としてlookat(インデックス)を進めることでトークナイズを行う(*char p; p++;みたいなことは気軽にできない)
	let len: usize = string.len();
	let mut lookat: usize = 0;
	let mut c: char;
	let mut slice: String;
	let string: Vec<char> = string.as_str().chars().collect::<Vec<char>>(); 

	


	while lookat < len {
		// 余白をまとめて飛ばす。streamを最後まで読んだならbreakする。
		match skipspace(&string, &mut lookat) {
			Ok(()) => {},
			Err(()) => {break;}
		}

		// 先に複数文字の演算子かどうかチェックする(文字数の多い方から)
		if lookat + 1 < len {
			slice = String::from_iter(string[lookat..lookat+2].iter());
			if slice == "==" || slice == "!=" || slice == ">=" || slice == "<=" {
				(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::ReservedTk, slice))));
				token_ptr_exceed(&mut token_ptr);

				lookat += 2;
				continue;
			}
		}

		// 単項演算子、括弧、代入演算子、文末のセミコロンを予約
		c = string[lookat];
		if c == '<' || c == '>' || c == '+' || c == '-' || c == '*' || c == '/' || c == '(' || c == ')' || c == '=' || c == ';' {
			(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::ReservedTk, c))));
			token_ptr_exceed(&mut token_ptr);

			lookat += 1;
			continue;
		}

		// 数字ならば、数字が終わるまでを読んでトークンを生成
		if is_digit(&c) {
			let num = strtol(&string, &mut lookat);
			(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::NumTk, num.to_string()))));
			token_ptr_exceed(&mut token_ptr);

			continue;
		}

		if is_return(&string, &mut lookat) {

			// トークン列にIdentTkとして追加する必要がある
			(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::ReturnTk, "return"))));
			token_ptr_exceed(&mut token_ptr);
			
			continue;
		}


		// 英字とアンダーバーを先頭とする文字を識別子としてサポートする
		if (c >= 'a' && c <= 'z') | (c >= 'A' && c <= 'Z') | (c == '_') {
			let name = read_lvar(&string, &mut lookat);

			// トークン列にIdentTkとして追加する必要がある
			(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::IdentTk, name))));
			token_ptr_exceed(&mut token_ptr);

			continue;
		}

		exit_eprintln!("トークナイズできません");
	}

	(*token_ptr).borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(Tokenkind::EOFTk, ""))));


	token_ptr_exceed(&mut token_head_ptr);

	token_head_ptr
}

/* ------------------------------------------------- トークナイズ用関数 ------------------------------------------------- */

// 空白を飛ばして読み進める
fn skipspace(string: &Vec<char>, index: &mut usize) -> Result<(), ()> {
	let limit = string.len();

	// 既にEOFだったならErrを即返す
	if *index >= limit {
		return Err(());
	}

	// 空白でなくなるまで読み進める
	while string[*index] == ' ' || string[*index] == '\n' ||  string[*index] == '\t' {
		*index += 1;
		if *index >= limit {
			return Err(());
		}
	}

	Ok(())
}

// 数字かどうかを判別する
fn is_digit(c: &char) -> bool{
	*c >= '0' && *c <= '9'
}



// 数字を読みつつindexを進める
fn strtol(string: &Vec<char>, index: &mut usize) -> u32 {
	let mut c = string[*index];
	let mut val = 0;
	let limit = string.len();

	// 数字を読む限りu32として加える
	while is_digit(&c) {
		val = val * 10 + (c.to_digit(10).unwrap() - '0'.to_digit(10).unwrap());
		*index += 1;

		// 最後に到達した場合は処理を終える
		if *index >= limit {
			return val;
		}
		c = string[*index];
	} 

	val
}

// 識別子の一部として使用可能な文字であるかどうかを判別する
fn canbe_ident_part (c: &char) -> bool {
	return (*c >= 'a' && *c <= 'z') | (*c >= 'A' && *c <= 'Z') | (*c >= '0' && *c <= '9') | (*c == '_')
}

// return文を読む
fn is_return(string: &Vec<char>, index: &mut usize) -> bool {
	// stringの残りにそもそもreturnの入る余地がなければそくreturn(index out of range回避)
	if string.len() < *index + 6 {return false;}

	// 1文字ずつみてreturnかどうか確認する(途中から6文字一気に見るのは難しいのと、Index out of rangeを避ける)
	//またreturn後の1文字は\n, \t, whitespace, ';'しか許容しない

	if string[*index] != 'r' {return false;}
	if string[*index+1] != 'e' {return false;}
	if string[*index+2] != 't' {return false;}
	if string[*index+3] != 'u' {return false;}
	if string[*index+4] != 'r' {return false;}
	if string[*index+5] != 'n' {return false;}

	// returnで';'なくコードが終わっているとどうせエラーになるのだが、そこはトークナイズではなく文法的な問題なのでparserに任せることにしてここではreturn trueにする
	if string.len() > *index + 6 {
		let next: char = string[*index+6];
		if next != ' ' && next != '\n' && next != '\t' {return false;}
	}

	// returnを読めたのでindexを進める
	*index += 6;
	true
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
		exit_eprintln!("数字であるべき位置で数字以外の文字\"{}\"が発見されました。", (**token_ptr).borrow().body.as_ref().unwrap());
	}
	let val = (**token_ptr).borrow().val.unwrap();

	token_ptr_exceed(token_ptr);
	
	val
}

// 次のトークンが識別子(変数など)であることを期待して次のトークンを読む関数
pub fn expect_ident(token_ptr: &mut Rc<RefCell<Token>>) -> String {
	if (**token_ptr).borrow().kind != Tokenkind::IdentTk {
		exit_eprintln!("識別子を期待した位置で\"{}\"が発見されました。", (**token_ptr).borrow().body.as_ref().unwrap());
	}
	let body = (**token_ptr).borrow_mut().body.as_ref().unwrap().clone();

	token_ptr_exceed(token_ptr);
	
	// 現段階では識別子として1文字しかサポートしない
	body
}

//  予約済みトークンを期待し、(文字列で)指定して読む関数(失敗するとexitする)
pub fn expect(token_ptr: &mut Rc<RefCell<Token>>, op: &str) {

	if (**token_ptr).borrow().kind != Tokenkind::ReservedTk{
		exit_eprintln!("予約されていないトークン\"{}\"が発見されました。", (**token_ptr).borrow().body.as_ref().unwrap());
	}
	if (**token_ptr).borrow().body.as_ref().unwrap() != op {
		exit_eprintln!("\"{}\"を期待した位置で\"{}\"が発見されました。", op, (**token_ptr).borrow().body.as_ref().unwrap());
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
	fn display_test_token() {
		let mut tmp_ptr;

		let mut token_ptr: Rc<RefCell<Token>> = tokenize("1+1-1".to_string());
		{
			println!("\ndisplay_test\n{}", "-".to_string().repeat(40));

			while !at_eof(&token_ptr) {
				println!("{}", (*token_ptr).borrow());
				tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			}
			println!("{}", (*token_ptr).borrow());
		}
	}


	#[test]
	fn tokenizer_test_1() {
		let mut tmp_ptr;
		let mut token_ptr: Rc<RefCell<Token>> = tokenize("1+1-1".to_string());

		{
			println!("\ntest1{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}	

	#[test]
	fn tokenizer_test_2() {

		let mut tmp_ptr;
		let mut token_ptr: Rc<RefCell<Token>> = tokenize("2*(1+1)-1".to_string());

		{
			println!("\ntest2{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			tmp_ptr = (*token_ptr).borrow().next.as_ref().unwrap().clone(); token_ptr = tmp_ptr;
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_3() {


		let mut token_ptr: Rc<RefCell<Token>> = tokenize("2*(1+1)-1 <= 2".to_string());
		{
			println!("\ntest3{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_4() {

		let mut token_ptr: Rc<RefCell<Token>> = tokenize("a = 1; a + 1;".to_string());
		{
			println!("\ntest4{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_5() {

		let mut token_ptr: Rc<RefCell<Token>> = tokenize("z = 1; z + 1;".to_string());
		{
			println!("\ntest5{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_6() {

		let mut token_ptr: Rc<RefCell<Token>> = tokenize("_abc = 1; def + 1; var22 = 22;".to_string());
		{
			println!("\ntest5{}", "-".to_string().repeat(40));

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::IdentTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::NumTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());

			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::ReservedTk);
			println!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
			
			token_ptr_exceed(&mut token_ptr);
			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_7(){
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(
			"local = -1;
			local_ = 2;
			local_1 = local_a = local;
			oops = 3;
			LOCAL = local * 30;
			local = (100 + 30 / 5 - 99) * (local > local);
			LOCAL + local*local + (LOCAL + local_)* local_1 + oops;".to_string()
		);

		{
			println!("\ntest5{}", "-".to_string().repeat(40));

			for _ in 0..59 {
				println!("token: {}", (*token_ptr).borrow().body.as_ref().unwrap());
				token_ptr_exceed(&mut token_ptr);
			}

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("OK: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}

	#[test]
	fn tokenizer_test_8(){
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(
			"a = 1; b = a * 8; return8 = 9; _return = 0; return 11;".to_string()
		);

		{
			println!("\ntest5{}", "-".to_string().repeat(40));

			for _ in 0..21 {
				println!("token: {}", (*token_ptr).borrow().body.as_ref().unwrap());
				token_ptr_exceed(&mut token_ptr);
			}

			assert_eq!((*token_ptr).borrow().kind, Tokenkind::EOFTk);
			eprintln!("Token: {}", (*token_ptr).borrow().body.as_ref().unwrap());
		}
	}
}
