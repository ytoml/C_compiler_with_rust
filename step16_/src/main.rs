use clap::Clap;
use std::fs::File;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};

mod tokenizer;
mod parser;
mod utils;
mod options;
mod generator;
use options::Opts;
use tokenizer::{Token, tokenize};
use parser::{program};
use generator::{gen, ASM};


fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);

		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(code);
		let node_heads = program(&mut token_ptr); // ここでLVAR_MAX_OFFSETがセットされる

		// 構文木が複数(関数の数)生成されているはずなのでそれぞれについて回す
		for node_ptr in node_heads {
			gen(&node_ptr);
		}

		// 最後に一気に書き込み
		println!("{}", *ASM.lock().unwrap());

    } else {
		// fileが指定されていない場合、exit
		exit_eprintln!("{}{}を指定してください。", "ソース", "ファイル");
    }

}


// 改行含め、コード全体を1つの文字列としてトークナイザに入れたい
fn code_concat(reader: BufReader<File>) -> String {
	let mut code = "".to_string();
	for line in reader.lines() {
		code += format!(" {}", line.unwrap()).as_str();
	}

	code
}


#[cfg(test)]
mod tests {
	use super::code_concat;
	use std::io::BufReader;
	use std::fs::File;
	use std::rc::Rc;
	use std::cell::RefCell;
	use crate::tokenizer::{Token, tokenize};
	use crate::parser::program;
	use crate::parser::tests::parse_stmts;

	#[test]
	fn code_concat_test() {
		let path = "./csrc/src.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);
		println!("{}", code);
	}

	#[test]
	fn tree_test() {
		let path = "./csrc/3stmt.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);
		println!("{}", code);

		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(code);
		let node_heads = parse_stmts(&mut token_ptr);
		println!("trees: {}", node_heads.len());
		assert_eq!(node_heads.len(), 3);
	}

	#[test]
	fn func_dec_test() {
		let path = "./csrc/func_dec.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);
		println!("{}", code);

		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(code);
		let node_heads = program(&mut token_ptr);
		println!("trees: {}", node_heads.len());
		assert_eq!(node_heads.len(), 2);
	}
}