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
use parser::{gen, expr};


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
            let mut asm = ".intel_syntax noprefix\n".to_string();
			asm += ".globl main\n";
			asm += "main:\n";

			// トークナイズしてトークンリストを生成したのち、構文木を生成
			let mut token_ptr: Rc<RefCell<Token>> = tokenize(line);
			let node_ptr = expr(&mut token_ptr);

			// 構文木からコードを生成(asmに追加)
			gen(&node_ptr, &mut asm);
            
			// 結果のpopとリターン命令を追加
			asm += "	pop rax\n";
            asm += "    ret\n";


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