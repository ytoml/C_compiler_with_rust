use clap::Clap;
use std::fs::File;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};

mod options;
mod token;
mod tokenizer;
mod node;
mod parser;
mod utils;
mod generator;
use generator::{gen, ASM};
use options::Opts;
use parser::program;
use token::Token;
use tokenizer::tokenize;
use utils::{CODES, FILE_NAMES};

fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f: File = File::open(path.as_str()).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader, path);
		
		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(0);
		let node_heads = program(&mut token_ptr);

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

// ファイルの情報を、グローバル変数の CODES と FILE_NAME に渡す
fn code_load(reader: BufReader<File>, file_name:impl Into<String>) {
	FILE_NAMES.lock().unwrap().push(file_name.into());
	let mut code = vec![];
	for line in reader.lines() {
		code.push(line.unwrap());
	}
	CODES.lock().unwrap().push(code);
}

#[cfg(test)]
mod tests {
	use super::code_load;
	use crate::utils::{CODES, FILE_NAMES};
	use std::io::BufReader;
	use std::fs::File;

	#[test]
	fn code_concat_test() {
		let path = "./csrc/src.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader,path);
		println!("{:#?}", CODES.lock().unwrap());
		println!("{:#?}", FILE_NAMES.lock().unwrap());
	}
}