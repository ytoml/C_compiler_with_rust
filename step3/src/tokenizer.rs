use crate::{exit_eprintln};

#[derive(Debug, PartialEq)]
pub enum Tokenkind {
	TK_RESERVED, // 記号
	TK_NUM, // 整数トークン
	TK_EOF, // 入力終わり
}

// ポインタ的にToken同士をつなぐのは諦めてVec<Token>で表現
pub struct Token {
	pub kind: Tokenkind,
	val: Option<i32>,  
	body: Option<String>
}

// 構造体にStringをうまく持たせるためのnewメソッド
impl Token {
	fn new(kind: Tokenkind, body: impl Into<String>) -> Token {
		let body = body.into();
		match kind {
			Tokenkind::TK_NUM => {
				// TK_NUMと共に数字以外の値が渡されることはないものとして、unwrapで処理
				let val = body.parse::<i32>().unwrap();
				Token {kind: kind, val: Some(val), body: Some(body)}
			},
			Tokenkind::TK_RESERVED => {
				Token {kind: kind, val: None, body: Some(body)}
			},
			Tokenkind::TK_EOF => {
				Token {kind: kind, val: None, body: Some("EOF".to_string())}
			}
		}
	}
}


// 入力文字列のトークナイズ
pub fn tokenize(string: String) -> Vec<Token> {
	// token_streamにTokenをプッシュしていく
	let mut token_stream = vec![];

	let len = string.len();
	let string = string.as_str().chars().collect::<Vec<char>>(); 

	let mut lookat = 0;
	let mut c;


	while lookat < len {
		// 余白をまとめて飛ばす。streamを最後まで読んだならbreakする。
		match skipspace(&string, &mut lookat) {
			Ok(()) => {},
			Err(()) => {break;}
		}

		c = string[lookat];
		if c == '+' || c == '-' {
			token_stream.push(
				Token::new(Tokenkind::TK_RESERVED, c)
			);
			lookat += 1;
			continue;
		}

		// 数字ならば、数字が終わるまでを読んでトークンを生成
		if isdigit(c) {
			let num = strtol(&string, &mut lookat);

			token_stream.push(
				Token::new(Tokenkind::TK_NUM, num.to_string())
			);
			continue;
		}
	}

	token_stream.push(
		Token::new(Tokenkind::TK_EOF, "")
	);

	token_stream
}

// 空白を飛ばして読み進める
fn skipspace(string: &Vec<char>, index: &mut usize) -> Result<(), ()> {
	let limit = string.len();

	// 既にEOFだったならErrを即返す
	if *index >= limit {
		return Err(());
	}

	// 空白でなくなるまで読み進める
	while string[*index] == ' ' {
		*index += 1;
		if *index >= limit {
			return Err(());
		}
	}


	Ok(())
}

// 数字かどうかを判別する
fn isdigit(c: char) -> bool{
	c >= '0' && c <=  '9'
}

// 数字を読みつつindexを進める
fn strtol(string: &Vec<char>, index: &mut usize) -> u32 {
	let mut c = string[*index];
	let mut val = 0;
	let limit = string.len();

	// 数字を読む限りu32として加える
	while isdigit(c) {
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



// 次のトークンが数字であることを期待して次のトークンを読む関数
pub fn expect_number(token_stream: &Vec<Token>, index: &mut usize) -> i32 {
	if token_stream[*index].kind != Tokenkind::TK_NUM {
		exit_eprintln!("数字であるべき位置で数字以外の文字\"{}\"が発見されました。", token_stream[*index].body.as_ref().unwrap());
	}
	let val = token_stream[*index].val.unwrap();

	// 読み位置を1つ前に進める
	*index += 1;
	
	val
}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとexitする)
pub fn expect(token_stream: &Vec<Token>, index: &mut usize, op: &str) {
	if token_stream[*index].kind != Tokenkind::TK_RESERVED{
		exit_eprintln!("予約されていないトークン\"{}\"が発見されました。", token_stream[*index].body.as_ref().unwrap());
	}
	if token_stream[*index].body.as_ref().unwrap() != op {
		exit_eprintln!("\"{}\"を期待した位置で\"{}\"が発見されました。", op, token_stream[*index].body.as_ref().unwrap());
	}
	// 読み位置を1つ前に進める
	*index += 1;
}


// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとfalseを返す)
pub fn consume(token_stream: &Vec<Token>, index: &mut usize, op: &str) -> bool {
	if token_stream[*index].kind != Tokenkind::TK_RESERVED || token_stream[*index].body.as_ref().unwrap() != op {
		return false;
	}

	// 読み位置を1つ前に進める
	*index += 1;
	
	true
}


// EOFかどうかを判断する関数
pub fn at_eof(token_stream: &Vec<Token>, index: &usize) -> bool{
	token_stream[*index].kind == Tokenkind::TK_EOF
}



