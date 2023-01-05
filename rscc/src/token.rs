use std::cell::RefCell;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::{exit_eprintln, utils::error_at};

pub type TokenRef = Rc<RefCell<Token>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tokenkind {
    Default,  // Default 用の kind
    Head,     // 先頭にのみ使用する kind
    Ident,    // 識別子
    Reserved, // 記号
    String,   // 文字列リテラル
    Num,      // 整数トークン
    Return,   // リターン
    Eof,      // 入力終わり
}

impl Display for Tokenkind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s: &str = match self {
            Tokenkind::Default => "Default Token",
            Tokenkind::Head => "Head Token",
            Tokenkind::Ident => "Identity Token",
            Tokenkind::Reserved => "Reserved Token",
            Tokenkind::String => "String Token",
            Tokenkind::Num => "Number Token",
            Tokenkind::Return => "Return Token",
            Tokenkind::Eof => "Eof Token",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub struct Token {
    pub kind: Tokenkind,
    pub val: Option<i32>,
    pub body: Option<String>,
    pub len: usize, // 1文字でないトークンもあるので、文字列の長さを保持しておく(非負)
    pub next: Option<TokenRef>, // Tokenは単純に単方向非循環LinkedListを構成することしかしないため、リークは起きないものと考える(循環の可能性があるなら、Weakを使うべき)

    // エラーメッセージ用
    pub file_num: usize,    // ファイルの番号
    pub line_num: usize,    // コード内の行数
    pub line_offset: usize, // 行内のオフセット
}

impl Default for Token {
    fn default() -> Token {
        Token {
            kind: Tokenkind::Default,
            val: None,
            body: None,
            len: 0,
            next: None,
            file_num: 0,
            line_num: 0,
            line_offset: 0,
        }
    }
}

impl Token {
    pub fn new(
        kind: Tokenkind,
        body: impl Into<String>,
        file_num: usize,
        line_num: usize,
        line_offset: usize,
    ) -> Token {
        let body: String = body.into();
        let len = body.chars().count();
        match kind {
            Tokenkind::Head => Token {
                kind,
                ..Default::default()
            },
            Tokenkind::Ident => Token {
                kind,
                body: Some(body),
                len,
                file_num,
                line_num,
                line_offset,
                ..Default::default()
            },
            Tokenkind::Num => {
                let val = body.parse::<i32>().unwrap();
                Token {
                    kind,
                    val: Some(val),
                    body: Some(body),
                    len,
                    next: None,
                    file_num,
                    line_num,
                    line_offset,
                }
            }
            Tokenkind::Reserved => Token {
                kind,
                body: Some(body),
                len,
                file_num,
                line_num,
                line_offset,
                ..Default::default()
            },
            Tokenkind::String => Token {
                kind,
                body: Some(body),
                len,
                file_num,
                line_num,
                line_offset,
                ..Default::default()
            },
            Tokenkind::Return => Token {
                kind,
                body: Some("return".to_string()),
                len: 6,
                file_num,
                line_num,
                line_offset,
                ..Default::default()
            },
            Tokenkind::Eof => Token {
                kind,
                body: Some("token of Eof".to_string()),
                ..Default::default()
            },
            _ => {
                panic!("invalid type of token.");
            } // Default を new で生成させない
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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

/// トークンのポインタを読み進める
#[inline]
pub fn token_ptr_exceed(token_ptr: &mut TokenRef) {
    let tmp_ptr = if let Some(ptr) = token_ptr.borrow().next.as_ref() {
        Rc::clone(ptr)
    } else {
        exit_eprintln!(
            "次のポインタを読めません。(現在のポインタのkind:{:?})",
            token_ptr.borrow().kind
        );
    };
    *token_ptr = tmp_ptr;
}

/// エラーメッセージ送出時に println! 等と同様の可変長引数を実現するためのマクロ
#[macro_export]
macro_rules! error_with_token {
	($fmt: expr, $tok: expr) => (
		use $crate::token::error_tok;
		error_tok($fmt, $tok);
	);

	($fmt: expr, $tok: expr, $($arg: tt)*) => (
		use $crate::token::error_tok;
		error_tok(format!($fmt, $($arg)*).as_str(), $tok);
	);
}

/// エラー送出のためのラッパー
pub fn error_tok(msg: &str, token: &Token) -> ! {
    // token.line_offset は token.len 以上であるはずなので負になる可能性をチェックしない
    error_at(
        msg,
        token.file_num,
        token.line_num,
        token.line_offset - token.len,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        println!("{}", Token::new(Tokenkind::Ident, "test", 0, 0, 0));
    }
}
