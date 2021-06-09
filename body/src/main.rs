use clap::Clap;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use anyhow::{Context, Result};

#[derive(Clap, Debug)]
#[clap(
    name = "rscc",
    version = "0.0.0",
    author = "Yuna Tomida",
    about = "C compiler with Rust for study"
)]

struct Opts {
    // verbose level
    #[clap(short, long)]
    verbose: bool,

    // 入力ファイル名
    #[clap(name = "FILE")]
    input_file: Option<String>,
}


fn main() {
    let opts = Opts::parse();
    
    // 入力ファイルが指定されていない場合、パニック
    if let Some(path) = opts.input_file {
        let f = File::open(path).unwrap();
        let reader = BufReader::new(f);

        for line in reader.lines() {

            let num_str = line.unwrap();

            match num_str.parse::<i32>() {
                Ok(num) => {
                    // ナイーブにアセンブラを出力
                    println!(".intel_syntax noprefix");
                    println!(".global main");
                    println!("main:");
                    println!("  mov rax, {}", num);
                    println!("  ret");
                    println!("");

                }
                Err(e) => {
                    println!("something wrong when parsing to i32{:?}", e);
                }
            }
        }
    } else {
        println!("please specify an input file to compile.");
    }

}

