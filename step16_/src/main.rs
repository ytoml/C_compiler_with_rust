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
use utils::CODE;

fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader);
		
		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize();
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
fn code_load(reader: BufReader<File>) {
	let mut code = CODE.lock().unwrap();
	for line in reader.lines() {
		code.push(line.unwrap());
	}
}

#[cfg(test)]
mod tests {
	use super::code_load;
	use crate::utils::CODE;
	use std::io::BufReader;
	use std::fs::File;

	#[test]
	fn code_concat_test() {
		let path = "./csrc/src.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		code_load(reader);
		println!("{:#?}", CODE.lock().unwrap());
	}
}