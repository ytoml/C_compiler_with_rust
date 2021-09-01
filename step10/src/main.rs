use clap::Clap;
use std::fs::File;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};

// tokenizerモジュールは未実装
mod tokenizer;
mod parser;
mod utils;
mod options;
mod generator;
use options::Opts;
use tokenizer::{Token, tokenize};
use parser::{program, LVAR_MAX_OFFSET};
use generator::gen;


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
		let node_heads = program(&mut token_ptr);

		// asmにアセンブリを文字列として追加していく
		let mut asm = ".intel_syntax noprefix\n".to_string();
		asm += ".globl main\n";
		asm += "main:\n";
		
		// プロローグ(変数の格納領域の確保)
		asm += "	push rbp\n";
		asm += "	mov rbp, rsp\n";
		asm += format!("	sub rsp, {}\n", LVAR_MAX_OFFSET.lock().unwrap()).as_str();
		
		// 構文木が複数(stmtの数)生成されているはずなのでそれぞれについて回す
		for node_ptr in node_heads {
			// 構文木からコードを生成(asmに追加)
			gen(&node_ptr, &mut asm);

			asm += "	pop rax\n";
		}

		// エピローグ(リターン処理)
		asm += "	mov rsp, rbp\n";
		asm += "	pop rbp\n";
		asm += "	ret\n";

		// 最後に一気に書き込み
		println!("{}", asm);

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
	use crate::parser::{program};

	#[test]
	fn code_concat_test() {
		let path = "./src.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);
		println!("{}", code);
	}

	#[test]
	fn tree_test() {
		let path = "./3stmt.txt";
		let f: File = File::open(path).unwrap();
        let reader: BufReader<File> = BufReader::new(f);
		let code: String = code_concat(reader);
		println!("{}", code);

		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(code);
		let node_heads = program(&mut token_ptr);
		println!("trees: {}", node_heads.len());
		assert_eq!(node_heads.len(), 3);


	}

}