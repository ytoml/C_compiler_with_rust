use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::{Display,  Formatter};
use std::fmt;

use crate::{
	exit_eprintln,
	utils::error_at,
};

pub type TokenRef = Rc<RefCell<Token>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tokenkind {
	DefaultTk,	// Default用のkind
	HeadTk,		// 先頭にのみ使用するkind
	IdentTk,	// 識別子
	ReservedTk,	// 記号
	StringTk,	// 文字列リテラル
	NumTk,		// 整数トークン
	ReturnTk,	// リターン
	EOFTk,		// 入力終わり
}

impl Display for Tokenkind {
	fn fmt(&self, f:&mut Formatter) -> fmt::Result {
		let s: &str;
		match self {
			Tokenkind::DefaultTk => {s = "Default Token";},
			Tokenkind::HeadTk => {s = "Head Token";},
			Tokenkind::IdentTk => {s = "Identity Token";},
			Tokenkind::ReservedTk => {s = "Reserved Token";},
			Tokenkind::StringTk => { s = "String Token"; }
			Tokenkind::NumTk => {s = "Number Token"; },
			Tokenkind::ReturnTk => {s = "Return Token"; }
			Tokenkind::EOFTk => {s = "EOF Token"; }
		}
		write!(f, "{}", s)
	}
}

// Rc<RefCell<T>>により共有可能な参照(ポインタ風)を持たせる
#[derive(Debug)]
pub struct Token {
	pub kind: Tokenkind,
	pub val: Option<i32>,  
	pub body: Option<String>,
	pub len: usize,							// 1文字でないトークンもあるので、文字列の長さを保持しておく(非負)
	pub next: Option<TokenRef>,	// Tokenは単純に単方向非循環LinkedListを構成することしかしないため、リークは起きないものと考える(循環の可能性があるなら、Weakを使うべき)

	// エラーメッセージ用
	pub file_num: usize,					// ファイルの番号
	pub line_num: usize,					// コード内の行数
	pub line_offset: usize,					// 行内のオフセット
}

impl Default for Token {
	fn default() -> Token {
		Token {kind: Tokenkind::DefaultTk, val: None, body: None, len: 0, next: None, file_num: 0, line_num:0, line_offset:0}
	}
}

// 構造体に String をうまく持たせるような new メソッド
impl Token {
	pub fn new(kind: Tokenkind, body: impl Into<String>, file_num: usize, line_num: usize, line_offset: usize) -> Token {
		let body: String = body.into();
		let len = body.chars().count();
		match kind {
			Tokenkind::HeadTk => {
				Token {kind: kind, .. Default::default()}
			}
			Tokenkind::IdentTk => {
				Token {
					kind: kind, 
					body: Some(body),
					len: len,
					file_num: file_num,
					line_num: line_num,
					line_offset: line_offset,
					.. Default::default()
				}
			}
			Tokenkind::NumTk => {
				// NumTk と共に数字以外の値が渡されることはないものとして、 unwrap で処理
				let val = body.parse::<i32>().unwrap();
				Token {
					kind: kind,
					val: Some(val),
					body: Some(body),
					len: len,
					next: None,
					file_num: file_num,
					line_num: line_num,
					line_offset: line_offset,
				}
			}
			Tokenkind::ReservedTk => {
				Token {
					kind: kind,
					body: Some(body),
					len: len,
					file_num: file_num,
					line_num: line_num,
					line_offset: line_offset,
					.. Default::default()
				}
			}
			Tokenkind::StringTk => {
				Token {
					kind: kind,
					body: Some(body),
					len: len,
					file_num: file_num,
					line_num: line_num,
					line_offset: line_offset,
					..Default::default()
				}
			}
			Tokenkind::ReturnTk => {
				Token {
					kind: kind, 
					body: Some("return".to_string()),
					len: 6,
					file_num: file_num,
					line_num: line_num,
					line_offset: line_offset,
					.. Default::default()
				}
			}
			Tokenkind::EOFTk => {
				Token {
					kind: kind, 
					body: Some("token of EOF".to_string()),
					.. Default::default()
				}
			}
			_ => { panic!("invalid type of token."); } // DefaultTkの場合(想定されていない)
		}
	}
}

impl Display for Token {
	fn fmt(&self, f:&mut Formatter) -> fmt::Result {
		let mut s = format!("{}\n", "-".to_string().repeat(40));
		s = format!("{}kind: {}\n", s, self.kind);
		s = format!("{}pos: [{}, {}]\n", s, self.line_num, self.line_offset);
		s = format!("{}length: {}\n", s, self.len);

		if let Some(e) = self.body.as_ref() {
			s = format!("{}body: {}\n", s, e);
		} else {
			s = format!("{}body: not exist\n", s);
		}

		if let Some(e) = self.val.as_ref() {
			s = format!("{}val: {}\n", s, e);
		} else {
			s = format!("{}val: -\n", s);
		}

		if let Some(e) = self.next.as_ref() {
			s = format!("{}next: -> {:?}\n", s, (**e).borrow().kind);
		} else {
			s = format!("{}next: -\n", s);
		}

		writeln!(f, "{}", s)
	}
}

// トークンのポインタを読み進める
#[inline]
pub fn token_ptr_exceed(token_ptr: &mut TokenRef) {
	let tmp_ptr;
	// next が None なら exit
	match token_ptr.borrow().next.as_ref() {
		Some(ptr) => {
			tmp_ptr = Rc::clone(ptr);
		},
		None => {
			exit_eprintln!("次のポインタを読めません。(現在のポインタのkind:{:?})", token_ptr.borrow().kind);
		}
	}
	*token_ptr = tmp_ptr;
}

// $tok は &Token を渡す
#[macro_export]
macro_rules! error_with_token {
	($fmt: expr, $tok: expr) => (
		use crate::token::error_tok;
		error_tok($fmt, $tok);
	);

	($fmt: expr, $tok: expr, $($arg: tt)*) => (
		use crate::token::error_tok;
		error_tok(format!($fmt, $($arg)*).as_str(), $tok);
	);
}

pub fn error_tok(msg: &str, token: &Token) -> ! {
	// token.line_offset は token.len 以上であるはずなので負になる可能性をチェックしない
	error_at(msg, token.file_num, token.line_num, token.line_offset-token.len);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
		println!("{}", Token::new(Tokenkind::IdentTk, "test", 0, 0, 0));
	}
}