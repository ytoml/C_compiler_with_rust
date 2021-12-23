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
use utils::{RED, LIGHTBLUE};
use asm::ASMCODE;
use generator::generate;
use options::Opts;
use parser::parse;
use tokenizer::tokenize;
use globals::{SRC, FILE_NAMES};

pub fn compile() -> String {
	let opts = Opts::parse();
	if let Some(path) = opts.input_file {
		match compile_src(path.as_str()) {
			Ok(asm) => { asm }
			Err(err) => { exit_eprintln!("{:#?}", err); }
		}
	} else {
		eprint!("\x1b[{}mrscc: \x1b[m\x1b[{}mError\x1b[m", LIGHTBLUE, RED);
		exit_eprintln!(" - ソースファイルを指定してください。");
	}
}

pub fn compile_src(path: &str) -> std::io::Result<String> {
	let f: File = File::open(path)?;
	let reader: BufReader<File> = BufReader::new(f);
	code_load(reader, path);
	run();
	Ok(ASMCODE.try_lock().unwrap().drain(..).collect::<String>())
}

/// ファイルの情報を、グローバル変数の SRC と FILE_NAME に渡す
fn code_load(reader: BufReader<File>, file_name:impl Into<String>) {
	FILE_NAMES.try_lock().unwrap().push(file_name.into());
	let mut code = vec!["".to_string()]; // コードの行の index を1始まりにするため空文字を入れておく
	for line in reader.lines() {
		// tokenizer の便利のため、各行の "\n" を復活させておく
		code.push(line.unwrap()+"\n");
	}
	SRC.try_lock().unwrap().push(code);
}

fn run() {
	let head = tokenize(0);
	let trees = parse(head);
	generate(trees);
}

#[cfg(test)]
mod tests {
	use std::io::BufReader;
	use std::fs::File;

	use crate::globals::{SRC, FILE_NAMES};
	use super::code_load;

	#[test]
	fn code_load_test() {
		let path = "./csrc/loadtest.c";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);

		code_load(reader,path);
		let src = SRC.try_lock().unwrap();
		let filenames = FILE_NAMES.try_lock().unwrap();
		assert_eq!(src.len(), 1);
		assert_eq!(src[0].len(), 67);
		assert_eq!(filenames.len(), 1);
		assert_eq!(filenames[0], path);
	}
}