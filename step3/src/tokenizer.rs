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

// 入力文字列のトークナイズ
pub fn tokenize(string: String) -> Token {
	let mut head = Token::new();
	head.next = None;

	// 未完成
	while  {

	}

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



