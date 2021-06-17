// トークンの種類
pub enum Tokenkind {
	TK_RESERVED, // 記号
	TK_NUM, // 整数トークン
	TK_EOF, // 入力終わり
}

// Rustでは再帰的な構造が所有権の関係で難しいのでどうするか…
pub struct Token {
	pub kind: Tokenkind,
	next: &Token,
	val: i32,  
	body: &str
}

// 次のトークンが数字であることを期待して次のトークンを読む関数
pub fn expect_number(token: Token) -> Restult<i32, ()>{}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとexitする)
pub fn expect(token: Token, op: String) -> Result<(), ()>{}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとfalseを返す)
pub fn consume(token: Token, op: String) -> bool {}

// EOFかどうかを判断する関数(しばらくは改行文字もEOFとみなす)
pub fn at_eof(token: Token) -> bool{}

// exitがうまく機能しない時の処理を入れ込むためにマクロを記述
#[macro_export]
macro_rules! expect_number {
	($token:expr) => {
		match expect_number($token) {
			Ok(num) => {num}
			Err(()) => {panic!();}
		}
	};
}

// exitがうまく機能しない時の処理を入れ込むためにマクロを記述
#[macro_export]
macro_rules! expect {
	($op:expr, $token:expr) => {
		match expect($op, $token) {
			Ok(()) => {}
			Err(()) => {panic!();}
		}
	};
}

