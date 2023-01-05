// トークナイザ
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::rc::Rc;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::{
    error_with_token,
    globals::SRC,
    token::{token_ptr_exceed, Token, TokenRef, Tokenkind},
    typecell::{Type, TypeCell},
    utils::{error_at, is_digit, strtol},
};

/// 入力文字列のトークナイズ
pub fn tokenize(file_num: usize) -> TokenRef {
    // Rcを使って読み進める
    let mut token_ptr: TokenRef = Rc::new(RefCell::new(Token::new(Tokenkind::Head, "", 0, 0, 0)));
    let mut token_head_ptr: TokenRef = Rc::clone(&token_ptr);
    let mut err_profile: (bool, usize, usize, &str) = (false, 0, 0, "");
    // error_at を使うタイミングで SRC のロックが外れているようにスコープを調整
    {
        let code = &mut SRC.try_lock().unwrap()[file_num];
        let mut is_block_comment = false;
        for (line_num, string) in code.iter().enumerate() {
            // StringをVec<char>としてlookat(インデックス)を進めることでトークナイズを行う(*char p; p++;みたいなことは気軽にできない)
            let mut lookat: usize = 0;
            let mut c: char;
            let string: Vec<char> = string.as_str().chars().collect::<Vec<char>>();
            let len: usize = string.len(); // Vec<char> にしてから len() を呼ぶことで、複数バイト文字も正しく1文字ずつ扱える

            while lookat < len {
                // 余白をまとめて飛ばす。streamを最後まで読んだならbreakする。
                match skipspace(&string, &mut lookat, len) {
                    Ok(()) => {}
                    Err(()) => {
                        break;
                    }
                }

                if is_block_comment {
                    if read(&string, "*/", &mut lookat, len) {
                        is_block_comment = false;
                    } else {
                        lookat += 1;
                    }
                    continue;
                }

                if read(&string, "/*", &mut lookat, len) {
                    is_block_comment = true;
                    continue;
                }

                if read(&string, "//", &mut lookat, len) {
                    break;
                }

                // 予約文字を判定
                if let Some(body) = is_reserved(&string, &mut lookat, len) {
                    token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                        Tokenkind::Reserved,
                        body,
                        file_num,
                        line_num,
                        lookat,
                    ))));
                    token_ptr_exceed(&mut token_ptr);
                    continue;
                }

                if is_return(&string, &mut lookat, len) {
                    // トークン列にIdentとして追加する必要がある
                    token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                        Tokenkind::Return,
                        "",
                        file_num,
                        line_num,
                        lookat,
                    ))));
                    token_ptr_exceed(&mut token_ptr);
                    continue;
                }

                // 数字ならば、数字が終わるまでを読んでトークンを生成
                c = string[lookat];
                if is_digit(&c) {
                    let num = strtol(&string, &mut lookat);
                    token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                        Tokenkind::Num,
                        num.to_string(),
                        file_num,
                        line_num,
                        lookat,
                    ))));
                    token_ptr_exceed(&mut token_ptr);
                    continue;
                }

                // 英字とアンダーバーを先頭とする文字を識別子としてサポートする
                if ('a'..='z').contains(&c) || ('A'..='Z').contains(&c) || c == '_' {
                    let name = read_lvar(&string, &mut lookat);

                    // トークン列にIdentとして追加する必要がある
                    token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                        Tokenkind::Ident,
                        name,
                        file_num,
                        line_num,
                        lookat,
                    ))));
                    token_ptr_exceed(&mut token_ptr);
                    continue;
                }

                // C ではソース上での文字列リテラルの改行は認められていないので、行ごとのループ内でリテラルを読む処理を完結させて良い
                let line_offset = lookat; // 文字列の先頭を指すように　line_offset を押さえておく
                match read_str_literal(&string, &mut lookat, len) {
                    Ok(literal) => {
                        if let Some(body) = literal {
                            token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                                Tokenkind::String,
                                body,
                                file_num,
                                line_num,
                                line_offset,
                            ))));
                            token_ptr_exceed(&mut token_ptr);
                            continue;
                        }
                    }
                    Err(msg) => {
                        err_profile = (true, line_num, lookat, msg);
                        break;
                    }
                }

                match read_char_literal(&string, &mut lookat, len) {
                    Ok(encoded) => {
                        if let Some(val) = encoded {
                            token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
                                Tokenkind::Num,
                                val.to_string(),
                                file_num,
                                line_num,
                                line_offset,
                            ))));
                            token_ptr_exceed(&mut token_ptr);
                            continue;
                        }
                    }
                    Err(msg) => {
                        err_profile = (true, line_num, lookat, msg);
                        break;
                    }
                }

                err_profile = (true, line_num, lookat, "トークナイズできません");
                break;
            }
            if err_profile.0 {
                break;
            }
        }
    }

    if err_profile.0 {
        error_at(err_profile.3, file_num, err_profile.1, err_profile.2);
    }

    token_ptr.borrow_mut().next = Some(Rc::new(RefCell::new(Token::new(
        Tokenkind::Eof,
        "",
        0,
        0,
        0,
    ))));
    token_ptr_exceed(&mut token_head_ptr);

    token_head_ptr
}

/* ------------------------------------------------- トークナイズ用関数 ------------------------------------------------- */
static QUAD_KEYWORDS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec!["else", "char"]));

static TRI_OPS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec!["<<=", ">>="]));

static TRI_KEYWORDS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec!["for", "int"]));

static BI_OPS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| {
    Mutex::new(vec![
        "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "++", "--", "+=", "-=", "*=", "/=", "%=",
        "&=", "^=", "|=",
    ])
});
static UNI_RESERVED: Lazy<Mutex<Vec<char>>> = Lazy::new(|| {
    Mutex::new(vec![
        ';', ',', '(', ')', '{', '}', '[', ']', '+', '-', '*', '/', '%', '&', '|', '^', '!', '~',
        '=', '<', '>',
    ])
});

static SPACES: Lazy<Mutex<Vec<char>>> = Lazy::new(|| Mutex::new(vec![' ', '\t', '\n']));

// 現在は int のみサポート
static TYPES: Lazy<Mutex<HashMap<String, Type>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let _ = map.insert(String::from("int"), Type::Int);
    let _ = map.insert(String::from("char"), Type::Char);
    Mutex::new(map)
});

// 空白を飛ばして読み進める
fn skipspace(string: &[char], index: &mut usize, len: usize) -> Result<(), ()> {
    // 既にEofだったならErrを即返す
    if *index >= len {
        return Err(());
    }

    // 空白でなくなるまで読み進める
    let spaces_access = SPACES.try_lock().unwrap();
    while spaces_access.contains(&string[*index]) {
        *index += 1;
        if *index >= len {
            return Err(());
        }
    }

    Ok(())
}

const QUOTE_ERROR_MSG: &str = "終わり引用符がありません。";

// 文字列リテラルを読む関数
fn read_str_literal(
    string: &[char],
    index: &mut usize,
    len: usize,
) -> Result<Option<String>, &'static str> {
    if *index >= len {
        return Ok(None);
    }
    if string[*index] != '\"' {
        return Ok(None);
    }

    let mut literal = vec![];
    *index += 1;
    while string[*index] != '\"' {
        literal.push(string[*index]);
        *index += 1;
        if *index >= len {
            return Err(QUOTE_ERROR_MSG);
        }
    }
    *index += 1;
    Ok(Some(literal.iter().collect()))
}

// char リテラルを読む関数
fn read_char_literal(
    string: &[char],
    index: &mut usize,
    len: usize,
) -> Result<Option<i32>, &'static str> {
    if *index >= len {
        return Ok(None);
    }
    if string[*index] != '\'' {
        return Ok(None);
    }

    let mut val = 0;
    *index += 1;
    // 各バイトを単に連結したものを int と見做して扱う(オーバーフローは無視する)
    while string[*index] != '\'' {
        let mut buf = vec![0u8; 4];
        for i in string[*index].encode_utf8(&mut buf).as_bytes() {
            val <<= 8;
            val += *i as i32;
        }
        *index += 1;
        if *index >= len {
            return Err(QUOTE_ERROR_MSG);
        }
    }
    *index += 1;
    Ok(Some(val))
}

fn read(string: &[char], read: impl Into<String>, index: &mut usize, len: usize) -> bool {
    let mut look = *index;
    let read: String = read.into();
    for c in read.chars() {
        if look >= len || c != string[look] {
            return false;
        }
        look += 1;
    }
    *index = look;

    true
}

// 識別子の一部として使用可能な文字であるかどうかを判別する
fn canbe_ident_part(c: &char) -> bool {
    ('a'..='z').contains(c) || ('A'..='Z').contains(c) || ('0'..='9').contains(c) || c == &'_'
}

// 予約されたトークンの後に空白なしで連続して良い文字であるかどうかを判別する。
fn can_follow_reserved(string: &[char], index: usize) -> bool {
    if let Some(c) = string.get(index) {
        if UNI_RESERVED.try_lock().unwrap().contains(c) || SPACES.try_lock().unwrap().contains(c) {
            return true;
        }
        return false;
    }
    // indexがout of bounds(=前のトークンが文末にある)ならトークナイズを許して後でパーサにエラーを出させる方針
    true
}

// 予約されたトークンだった場合はSome(String)を返す
fn is_reserved(string: &[char], index: &mut usize, len: usize) -> Option<String> {
    // 先に複数文字の演算子かどうかチェックする(文字数の多い方から)
    let lim = *index + 6;
    if lim <= len {
        let slice: String = String::from_iter(string[*index..lim].iter());
        if slice == "sizeof" && can_follow_reserved(string, lim) {
            *index = lim;
            return Some(slice);
        }
    }

    let lim = *index + 5;
    if lim <= len {
        let slice: String = String::from_iter(string[*index..lim].iter());
        if slice == "while" && can_follow_reserved(string, lim) {
            *index = lim;
            return Some(slice);
        }
    }

    let lim = *index + 4;
    if lim <= len {
        let slice: String = String::from_iter(string[*index..lim].iter());
        if QUAD_KEYWORDS.try_lock().unwrap().contains(&slice.as_str())
            && can_follow_reserved(string, lim)
        {
            *index = lim;
            return Some(slice);
        }
    }

    let lim = *index + 3;
    if lim <= len {
        let slice: String = String::from_iter(string[*index..lim].iter());
        if TRI_OPS.try_lock().unwrap().contains(&slice.as_str())
            || TRI_KEYWORDS.try_lock().unwrap().contains(&slice.as_str())
                && can_follow_reserved(string, lim)
        {
            *index = lim;
            return Some(slice);
        }
    }

    // 2文字演算子とif
    let lim = *index + 2;
    if lim <= len {
        let slice: String = String::from_iter(string[*index..(*index + 2)].iter());
        if BI_OPS.try_lock().unwrap().contains(&slice.as_str())
            || (slice == "if" && can_follow_reserved(string, lim))
        {
            *index = lim;
            return Some(slice);
        }
    }

    // 単項演算子、括弧、代入演算子、文末のセミコロンを予約
    if *index < len {
        let c: char = string[*index];

        if UNI_RESERVED.try_lock().unwrap().contains(&c) {
            *index += 1;
            return Some(c.to_string());
        }
    }

    None
}

// return文を読む
fn is_return(string: &[char], index: &mut usize, len: usize) -> bool {
    // is_reservedと同じ要領でreturnを読み取る
    let lim = *index + 6;
    // stringの残りにそもそもreturnの入る余地がなければ即return(index out of range回避)
    if lim >= len {
        return false;
    }

    let slice: String = String::from_iter(string[*index..lim].iter());
    if slice == "return" && can_follow_reserved(string, lim) {
        *index = lim;
        true
    } else {
        false
    }
}

// LVarに対応する文字列を抽出しつつ、indexを進める
fn read_lvar(string: &[char], index: &mut usize) -> String {
    let mut name = "".to_string();

    // 1文字ずつみて連結する
    while canbe_ident_part(&string[*index]) {
        name = format!("{}{}", name, string[*index]);
        *index += 1;
    }

    name
}

/* ------------------------------------------------- トークン処理用関数(parserからの呼び出しを含むためpubが必要) ------------------------------------------------- */

// is: 次の Token がある性質のものであるかを判定(-> bool)
// consume: 次の Token がある性質のものであるかを判定(-> Option<_>)
// expect: 次の Token がある性質のものであるかを判定、違う場合は error

#[inline]
pub fn is(token_ptr: &mut TokenRef, op: &str) -> bool {
    token_ptr.borrow().kind == Tokenkind::Reserved
        && token_ptr.borrow().body.as_ref().unwrap() == op
}

// 期待する次のトークンを(文字列で)指定して読む関数(失敗するとfalseを返す)
#[inline]
pub fn consume(token_ptr: &mut TokenRef, op: &str) -> bool {
    if is(token_ptr, op) {
        token_ptr_exceed(token_ptr);
        true
    } else {
        false
    }
}

#[inline]
pub fn expect(token_ptr: &mut TokenRef, op: &str) {
    if !consume(token_ptr, op) {
        error_with_token!(
            "\"{}\"を期待した位置で予約されていないトークン\"{}\"が発見されました。",
            &*token_ptr.borrow(),
            op,
            token_ptr.borrow().body.as_ref().unwrap()
        );
    }
}

#[inline]
fn is_number(token_ptr: &mut TokenRef) -> bool {
    token_ptr.borrow().kind == Tokenkind::Num
}

#[inline]
pub fn consume_number(token_ptr: &mut TokenRef) -> Option<i32> {
    if is_number(token_ptr) {
        let val = token_ptr.borrow().val.unwrap();
        token_ptr_exceed(token_ptr);
        Some(val)
    } else {
        None
    }
}

#[inline]
pub fn expect_number(token_ptr: &mut TokenRef) -> i32 {
    if let Some(val) = consume_number(token_ptr) {
        val
    } else {
        error_with_token!(
            "数字であるべき位置で数字以外の文字\"{}\"が発見されました。",
            &*token_ptr.borrow(),
            token_ptr.borrow().body.as_ref().unwrap()
        );
    }
}

#[inline]
fn is_ident(token_ptr: &mut TokenRef) -> bool {
    token_ptr.borrow().kind == Tokenkind::Ident
}

#[inline]
pub fn consume_ident(token_ptr: &mut TokenRef) -> Option<String> {
    if is_ident(token_ptr) {
        let body = token_ptr.borrow().body.clone().unwrap();
        token_ptr_exceed(token_ptr);
        Some(body)
    } else {
        None
    }
}

#[inline]
pub fn expect_ident(token_ptr: &mut TokenRef) -> String {
    if let Some(body) = consume_ident(token_ptr) {
        body
    } else {
        error_with_token!(
            "識別子を期待した位置で\"{}\"が発見されました。",
            &*token_ptr.borrow(),
            token_ptr.borrow().body.as_ref().unwrap()
        );
    }
}

#[inline]
pub fn is_type(token_ptr: &mut TokenRef) -> bool {
    is_kind(token_ptr, Tokenkind::Reserved)
        && TYPES
            .try_lock()
            .unwrap()
            .contains_key(token_ptr.borrow().body.as_ref().unwrap())
}

#[inline]
pub fn consume_type(token_ptr: &mut TokenRef) -> Option<TypeCell> {
    if is_type(token_ptr) {
        let base: Type = *TYPES
            .try_lock()
            .unwrap()
            .get(token_ptr.borrow().body.as_ref().unwrap())
            .unwrap();
        token_ptr_exceed(token_ptr);
        Some(TypeCell::new(base))
    } else {
        None
    }
}

#[inline]
pub fn expect_type(token_ptr: &mut TokenRef) -> TypeCell {
    if let Some(typ) = consume_type(token_ptr) {
        typ
    } else {
        error_with_token!("型の指定が必要です。", &*token_ptr.borrow());
    }
}

#[inline]
pub fn is_kind(token_ptr: &mut TokenRef, kind: Tokenkind) -> bool {
    token_ptr.borrow().kind == kind
}

#[inline]
pub fn consume_kind(token_ptr: &mut TokenRef, kind: Tokenkind) -> bool {
    if is_kind(token_ptr, kind) {
        token_ptr_exceed(token_ptr);
        true
    } else {
        false
    }
}

#[inline]
pub fn consume_literal(token_ptr: &mut TokenRef) -> Option<String> {
    if is_kind(token_ptr, Tokenkind::String) {
        let literal = token_ptr.borrow().body.clone().unwrap();
        token_ptr_exceed(token_ptr);
        Some(literal)
    } else {
        None
    }
}

#[inline]
pub fn expect_literal(token_ptr: &mut TokenRef) -> String {
    if let Some(literal) = consume_literal(token_ptr) {
        literal
    } else {
        error_with_token!(
            "文字列リテラルを期待した位置で予約されていないトークン\"{}\"が発見されました。",
            &*token_ptr.borrow(),
            token_ptr.borrow().body.as_ref().unwrap()
        );
    }
}

#[inline]
pub fn at_eof(token_ptr: &TokenRef) -> bool {
    token_ptr.borrow().kind == Tokenkind::Eof
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::{FILE_NAMES, SRC};

    fn test_init(src: &str) {
        let mut src_: Vec<String> = src.split('\n').map(|s| s.to_string() + "\n").collect();
        FILE_NAMES.try_lock().unwrap().push("test".to_string());
        let mut code = vec!["".to_string()];
        code.append(&mut src_);
        SRC.try_lock().unwrap().push(code);
    }

    #[test]
    fn lvar() {
        let src: &str = "
			int local, local_, local_1, local_a, oops, LOCAL;
			local = -1;
			local_ = 2;
			local_1 = local_a = local;
			oops = 3;
			LOCAL = local * 30;
			local = (100 + 30 / 5 - 99) * (local > local);
			LOCAL + local*local + (LOCAL + local_)* local_1 + oops;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn return_() {
        let src: &str = "
			int a, b, return8, _return;
			a = 1;
			b = a * 8;
			return8 = 9;
			_return = 0;
			return 11;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn ctrl() {
        let src: &str = "
			int i, x, if_, return8;
			for( i = 10; ;  ) i = i + 1;
			x = 20;
			while(i == 0) x = x + 1;
			if_ = 10
			if(if_ >= 0) if_ - 100; else if_ * 100;
			return8 = 10;
			return return8;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn block() {
        let src: &str = "
			int i;
			for( i = 10; ; ) {i = i + 1;}
			{}
			{i = i + 1;}
			return 10;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn addr_deref() {
        let src: &str = "
			int x, y, z;
			x = 4;
			y = &x;
			z = &y;
			return *&**z;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn ops() {
        let src: &str = "
			int x, y, z, w, p;
			x = 1;
			y = 0;
			z = 2;
			if( x || (y && z)) print_helper(x); else return z;
			w = x & y ^ z;
			p = !x;
			return ~z;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn ops2() {
        let src: &str = "
			int x, y, z;
			x = 1;
			y = 0;
			z = 2;
			if( x || (y && z)) print_helper(x); else return z;
			w = x & y ^ z;
			p = !x;
			return ~z;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn ops3() {
        let src: &str = "
			int x;
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
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn sizeof() {
        let src: &str = "
			sizeof x;
			sizeofx;
			sizeof 1;
			sizeof(x+y);
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn array() {
        let src: &str = "
			int x[20];
			x[5] = 20;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn char_() {
        let src: &str = "
			char c;
			char c[10];
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn str_literal() {
        let src: &str = "
			char *c =
			\"This is a test of string literal\";
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn char_literal() {
        let src: &str = "
			char c = \'a\';
			int x = \'ああ\'
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }

    #[test]
    fn comment() {
        let src: &str = "
			// This is a comment.
			/* block comment
			*/
			int a //*
			// */ b;
			a = 100/*
			*// 5;
		";
        test_init(src);

        let mut token_ptr: TokenRef = tokenize(0);
        while token_ptr.borrow().kind != Tokenkind::Eof {
            println!("{}", token_ptr.borrow());
            token_ptr_exceed(&mut token_ptr);
        }
        assert_eq!(token_ptr.borrow().kind, Tokenkind::Eof);
        println!("{}", token_ptr.borrow());
    }
}
