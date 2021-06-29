use clap::Clap;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use anyhow::{Context, Result};

// tokenizerモジュールは未実装
mod tokenizer;
mod utils;
mod options;
use options::Opts;
use tokenizer::{Token, tokenize, expect, expect_number, consume, at_eof};


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

			// よく考えたら参照で渡す意味そんなないかも
			let token_stream = tokenize(&line);
			let mut lookat: usize = 0;

			// 頭は数字から入ることを想定
			let num = expect_number(&token_stream, &mut lookat);
			asm += format!("    mov rax, {}\n", num).as_str();

			// トークンを処理
			loop {
				if consume(&token_stream, &mut lookat,  "+") {
					let num = expect_number(&token_streamm, &mut lookat);
					asm += format!("    add rax, {}\n", num).as_str();
					continue;
				}

				// +でなければ-を期待して処理
				expect(&token_stream, &mut lookat,  "-");
				asm += format!("    sub rax, {}\n", num).as_str();
				

				// EOF到達でloopを抜ける 
				if at_eof(&token_stream, &lookat) {break;}
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