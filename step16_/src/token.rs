use crate::{exit_eprintln};
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::{Display,  Formatter};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Tokenkind {
	DefaultTk,	// Default用のkind
	HeadTk,		// 先頭にのみ使用するkind
	IdentTk,	// 識別子
	ReservedTk,	// 記号
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
			Tokenkind::NumTk => {s = "Number Token";},
			Tokenkind::ReturnTk => {s = "Return Token";}
			Tokenkind::EOFTk => {s = "EOF Token";}
		}
		write!(f, "{}", s)
	}
}

// Rc<RefCell<T>>により共有可能な参照(ポインタ風)を持たせる
pub struct Token {
	pub kind: Tokenkind,
	pub val: Option<i32>,  
	pub body: Option<String>,
	pub len: usize,							// 1文字でないトークンもあるので、文字列の長さを保持しておく(非負)
	pub next: Option<Rc<RefCell<Token>>>,	// Tokenは単純に単方向非循環LinkedListを構成することしかしないため、リークは起きないものと考える(循環の可能性があるなら、Weakを使うべき)
}

impl Default for Token {
	fn default() -> Token {
		Token {kind: Tokenkind::DefaultTk, val: None, body: None, len: 0, next: None}
	}
}

// 構造体に String をうまく持たせるような new メソッド
impl Token {
	pub fn new(kind: Tokenkind, body: impl Into<String>) -> Token {
		let body: String = body.into();
		let len = body.chars().count();
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
				// NumTk と共に数字以外の値が渡されることはないものとして、 unwrap で処理
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

	// next が None なら exit
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn display() {
		println!("{}", Token::new(Tokenkind::IdentTk, "test"));
	}
}