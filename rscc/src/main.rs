use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::Clap;

mod asm;
mod generator;
mod globals;
mod initializer;
mod node;
mod options;
mod parser;
mod token;
mod tokenizer;
mod typecell;
mod utils;
use asm::ASMCODE;
use generator::generate;
use options::Opts;
use parser::parse;
use tokenizer::tokenize;
use globals::{CODES, FILE_NAMES};

fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f: File = File::open(path.as_str()).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader, path);
		run_cc();

		// 最後に一気に書き込み
		print!("{}", ASMCODE.try_lock().unwrap());
    } else {
		// fileが指定されていない場合、exit
		exit_eprintln!("{}{}を指定してください。", "ソース", "ファイル");
    }
}

// ファイルの情報を、グローバル変数の CODES と FILE_NAME に渡す
fn code_load(reader: BufReader<File>, file_name:impl Into<String>) {
	FILE_NAMES.try_lock().unwrap().push(file_name.into());
	let mut code = vec!["".to_string()]; // コードの行の index を1始まりにするため空文字を入れておく
	for line in reader.lines() {
		// tokenizer の便利のため、各行の "\n" を復活させておく
		code.push(line.unwrap()+"\n");
	}
	CODES.try_lock().unwrap().push(code);
}

fn run_cc() {
	let head = tokenize(0);
	let trees = parse(head);
	generate(trees);
}

#[cfg(test)]
mod tests {
	use std::io::BufReader;
	use std::fs::File;

	use crate::globals::{CODES, FILE_NAMES};
	use super::code_load;

	#[test]
	fn code_concat_test() {
		let path = "./csrc/src.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader,path);
		println!("{:#?}", CODES.try_lock().unwrap());
		println!("{:#?}", FILE_NAMES.try_lock().unwrap());
	}
}