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
use tokenizer::{Token, tokenize, expect, expect_number, consume, at_eof};
// use tokenizer::{Token, tokenize, expect, expect_number, consume, at_eof};
use parser::*;


fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されているかどうかで分岐
    if let Some(path) = opts.input_file {
        let f = File::open(path).unwrap();
        let reader = BufReader::new(f);

		// 1行ごとに処理
        for line in reader.lines() {
            // このループ内でlineを式として解釈していく(以降のバージョンではこの部分の変数名はlineに統一する)
            let line = line.unwrap();
            let mut asm = ".intel_syntax noprefix\n.globl main\nmain:\n".to_string();

			let mut token_ptr: Rc<RefCell<Token>> = tokenize(line);

			// 頭は数字から入ることを想定
			let num = expect_number(&mut token_ptr);
			asm += format!("    mov rax, {}\n", num).as_str();


			// EOFまでトークンを処理
			while !at_eof(&token_ptr) {
				if consume(&mut token_ptr, "+") {
					let num = expect_number(&mut token_ptr);
					asm += format!("    add rax, {}\n", num).as_str();
					continue;
				}

				// +でなければ-を期待して処理
				expect(&mut token_ptr, "-");
				let num = expect_number(&mut token_ptr);
				asm += format!("    sub rax, {}\n", num).as_str();

			}

            
			// リターン命令を加える
            asm += format!("    ret").as_str();


            // 最後に一気に書き込み
            println!("{}", asm);

            // 序盤はファイルの最初1行だけを解釈する
			break;
        }
    } else {
		// fileが指定されていない場合、exit
		exit_eprintln!("{}{}を指定してください。", "ソース", "ファイル");
    }

}