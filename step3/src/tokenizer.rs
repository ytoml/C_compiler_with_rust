// トークンの種類
use anyhow::{Context, Result};

pub enum Tokenkind {
	TK_RESERVED, // 記号
	TK_NUM, // 整数トークン
	TK_EOF, // 入力終わり
}

// Rustでは再帰的な構造が所有権の関係で難しいのでどうするか…
pub struct Token {
	pub kind: Tokenkind,
	val: Some(i32),  
	body: Some(String)
}

impl Token {
	pub fn new(kind: Tokenkind, body: impl Into<String>) -> Result<Token> {
		let body = body.into();
		match kind {
			Tokenkind::TK_NUM => {
				let val = body.parse::<i32>().with_context(|| format!("\"{}\"をi32に変換できませんでした。", body))?;
				Token {kind: kind, val: Some(val), body: Some(body)}
			},
			Tokenkind::TK_RESERVED => {
				Token {kind: kind, val: None, body: Some(body)}
			},
			Tokenkind::TK_EOF => {
				Token {kind: kind, val: None, body: None}
			},
			_ => {panic!("Tokenkindが正しく設定されていません")}
		}
	}
}


// 入力文字列のトークナイズ
pub fn tokenize(string: &String) -> Vec<Token> {
	let token_stream = vec![];
	head.next = None;

	// 未完成
	let string = line.as_str().chars(); 

	let mut lookat = 0;
	let mut c;
	while lookat < string.len() {
		// 余白をまとめて飛ばす。streamを最後まで読んだならbreakする。
		match skipspace(string, lookat) {
			Ok(num) => {lookat = num;},
			Err(()) => {break;}
		}

		c = string[lookat];
		if (c == '+' || c == '-') {
			token_stream.push(
				Token::new(Tokenkind::TK_RESERVED, c)
			);
			continue;
		}

		// 数字ならば、数字が終わるまでを読んでトークンを生成
		if (isdigit(c)) {
			let val;
			(val, lookat) = strtol(string, lookat);
			token_stream.push(
				Token::new(Tokenkind::TK_NUM, c)
			);
			continue;
		}


	}
}

// 空白を飛ばして読み進める
fn skipspace(string: &Vec<char>, index: i32) -> Result<i32, ()> {
	let limit = string.len();
	while string[index] == ' ' {
		index += 1;
		if index >= limit {
			Err(())
		}
	}

	Ok(index)
}

// 数字かどうかを判別する
fn isdigit(c: char) {
	c >= '0' && c <=  '9'
}

// 戻り値は読んだ数字と読んだ後のindex
fn strtol(string: &Vec<char>, index: i32) -> (i32, i32) {
	let mut c = string[index];
	let mut val = 0;

	// 数字を読む限りi32として加える
	while c >= '0' && c <=  '9' {
		val = val * 10 + (c.to_digit(10).unwrap() - '0'.to_digit(10).unwrap());
		index += 1;
		c = string[index];
	} 

	(val, index)

}



// 次のトークンが数字であることを期待して次のトークンを読む関数
pub fn expect_number(token: Token) -> i32 {
	if token.kind != Tokenkind::TK_NUM {
		exit_eprintln!("数字であるべき位置で数字以外の文字\"{}\"が発見されました。", token.body);
	}
	let val = token.val;
	// token.toNext();
	val
}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとexitする)
pub fn expect(token: Token, op: String) {
	if token.kind != Tokenkind::TK_RESERVED{
		exit_eprintln!("予約されていないトークン\"{}\"が発見されました。", token.body);
	}
	if token.body[0].to_string() != op {
		exit_eprintln!("\"{}\"を期待した位置で\"{}\"が発見されました。", op, token.body);
	}
	// token.toNext();
}


// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとfalseを返す)
pub fn consume(token: Token, op: String) -> bool {
	if token.kind != Tokenkind::TK_RESERVED || token.body[0].to_string() != op {
		false
	}
	// token.toNext();
	true
}

// EOFかどうかを判断する関数(しばらくは改行文字もEOFとみなす)
pub fn at_eof(token: Token) -> bool{
	token.kind == Tokenkind::TK_EOF
}



