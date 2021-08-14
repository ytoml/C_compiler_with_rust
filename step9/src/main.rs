use clap::Clap;
use std::fs::File;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{stdin, BufRead, BufReader};
use anyhow::{Context, Result};

// tokenizerモジュールは未実装
mod tokenizer;
mod parser;
mod utils;
mod options;
use options::Opts;
use tokenizer::{Token, tokenize};
use parser::{gen, program};


fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f = File::open(path).unwrap();
        let reader = BufReader::new(f);

		// 改行含め、コード全体を1つの文字列としてトークナイザに入れたい
		let mut code = "".to_string();
		for line in reader.lines() {
			code += format!(" {}", line.unwrap()).as_str();
		}

		// asmに
		let mut asm = ".intel_syntax noprefix\n".to_string();
		asm += ".globl main\n";
		asm += "main:\n";

		// トークナイズしてトークンリストを生成したのち、構文木を生成
		let mut token_ptr: Rc<RefCell<Token>> = tokenize(code);
		
		let node_heads = program(&mut token_ptr);
		let mut asm = "".to_string();
		// 構文木が複数(stmtの数)生成されているはずなのでそれぞれについて回す
		for node_ptr in node_heads {
			// 構文木からコードを生成(asmに追加)
			gen(&node_ptr, &mut asm);

			asm += "	pop rax\n";
		}

		// 結果のpopとリターン命令を追加
		asm += "	pop rax\n";
		asm += "	ret\n";


		// 最後に一気に書き込み
		println!("{}", asm);

    } else {
		// fileが指定されていない場合、exit
		exit_eprintln!("{}{}を指定してください。", "ソース", "ファイル");
    }

}