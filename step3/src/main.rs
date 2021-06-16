use clap::Clap;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use anyhow::{Context, Result};

// tokenizerモジュールは未実装
// mod tokenizer;
mod utils;
mod options;
use options::Opts;


fn main() {
    // 引数の処理
    let opts = Opts::parse();
    
    // 入力ファイルが指定されていない場合、パニック
    if let Some(path) = opts.input_file {
        let f = File::open(path).unwrap();
        let reader = BufReader::new(f);

        for line in reader.lines() {
            // このループ内でlineを式として解釈していく(以降のバージョンではこの部分の変数名はlineに統一する)
            let line = line.unwrap();
            
            // 先頭から1文字ずつ値を読む。読み込んだ値が数値ならregに入れていき、そうでなければ演算子とみなす
            // flagは、1つ前に演算子を読んだかを判定する。asmは出力する文字列(アセンブラコード)。
            // 頭に+,-がくることについては許し、regが0なら計算を飛ばすという形で対処
            let mut reg = 0;
            let mut flag = false;
            let mut asm = ".intel_syntax noprefix\n.globl main\nmain:\n".to_string();
            let mut op_prev: char = ' ';
            for c in line.as_str().chars() {
                // 空白は無視
                if c == ' ' {continue;}

                // 数値ならそれまで読んだ結果を1桁繰り上げてから足す
                if c >= '0' && c <= '9' {

                    reg = reg * 10 + (c.to_digit(10).unwrap() - '0'.to_digit(10).unwrap());
                    if flag {flag = false}

                } else {
                    // それ以外は演算子として扱う

                    if flag {
                        exit_eprintln!("\"{}\":演算子が連続しています。", c);
                    }

                    if reg == 0 {
                        flag = true;
                        continue;
                    }

                    match op_prev {
                        '+' => {
                            asm += format!("    add rax, {}\n", reg).as_str();
                        }
                        '-' => {
                            asm += format!("    sub rax, {}\n", reg).as_str();
                        }
                        ' ' => {
                            asm += format!("    mov rax, {}\n", reg).as_str();
                        }
                        _ => {
                            exit_eprintln!("\"{}\":演算子として不正です。", op_prev);
                        }
                    }
                    op_prev = c;
                    // 読み込む整数のリセット及び演算子読みましたよフラグ
                    reg = 0;
                    flag = true;
                }
            }
            // 演算子で終わる場合、エラー
            if flag {
                exit_eprintln!("式が演算子で終了しています。");
            } 

            if reg != 0 {
                // あまり綺麗でないが、文末で最後の数字に対する計算を加える必要がある
                match op_prev {
                    '+' => {
                        asm += format!("    add rax, {}\n", reg).as_str();
                    }
                    '-' => {
                        asm += format!("    sub rax, {}\n", reg).as_str();
                    }
                    _ => {
                        exit_eprintln!("\"{}\":演算子として不正です。", op_prev);
                    }
                }
            }
            asm += format!("    ret").as_str();


            // 最後に一気に書き込み
            println!("{}", asm);

            // 最初はひとまずソースの一行目のみを受け取るのみにしておく。
            break;
        }
    } else {
		exit_eprintln!("{}を指定してください。", "ソースファイル");
    }

}
