use crate::globals::{CODES, FILE_NAMES};

// Errorの報告をするマクロ(ほぼeprint!のラッパ)
// これを使う際は使う側でuseが必要なことに注意
#[macro_export]
macro_rules! exit_eprint {
	// 引数なし
	() => {
		std::process::exit(1);
	};
	// 文字列リテラルのみの引数
	($fmt: expr) => {
		eprint!($fmt);
		std::process::exit(1);
	};

	// 第二引数以降がある場合 
	($fmt: expr, $($arg: tt)*) =>{
		eprint!($fmt, $($arg)*);
		std::process::exit(1);
	};
}

// eprintln!のラッパ
#[macro_export]
macro_rules! exit_eprintln {
	// 引数なし
	() => {
		eprint!("\n");
		std::process::exit(1);
	};
	// 文字列リテラルのみの引数
	($fmt: expr) => {
		eprint!(concat!($fmt, "\n"));
		std::process::exit(1);
	};

	// 第二引数以降がある場合 
	($fmt: expr, $($arg: tt)*) =>(
		eprint!(concat!($fmt, "\n"),$($arg)*);
		std::process::exit(1);
	);
}

// 数字かどうかを判別する
pub fn is_digit(c: &char) -> bool{
	*c >= '0' && *c <= '9'
}

// 数字を読みつつindexを進める
pub fn strtol(string: &Vec<char>, index: &mut usize) -> u32 {
	let mut c = string[*index];
	let mut val = 0;
	let limit = string.len();

	// 数字を読む限りu32として加える
	while is_digit(&c) {
		val = val * 10 + (c.to_digit(10).unwrap() - '0'.to_digit(10).unwrap());
		*index += 1;

		// 最後に到達した場合は処理を終える
		if *index >= limit {
			return val;
		}
		c = string[*index];
	} 
	val
}

// エラー位置を報告し、exit_eprintln! する関数
const RED: usize = 31;
const LIGHTBLUE: usize = 36;
pub fn error_at(msg: &str, file_num: usize, line_num: usize, line_offset: usize) -> ! {
	// ファイル名には今のところこの関数でしかアクセスしないので、デッドロックの検査はしない
	let file_name = &FILE_NAMES.lock().unwrap()[file_num];

	match CODES.try_lock() {
		Ok(codes) => {
			let code_line = &codes[file_num][line_num];
			let all_space = code_line.chars().map(|c| if c == '\t' {'\t'} else {' '}).collect::<String>();
			let space = &all_space[..line_offset];
			eprintln!("\x1b[{}mRSCC: Compile Error\x1b[m", RED);
			eprintln!("\x1b[{}m{}:{}:{}\x1b[m", LIGHTBLUE, file_name, line_num, line_offset);
			eprint!("{}", code_line); // code_line には \n が含まれるので eprint! を使う
			exit_eprintln!("{}\x1b[{}m^\x1b[m {}", space, RED, msg);
		}
		// ここのエラーが出ないように CODES の lock をとった状態でエラー関係の関数やマクロを呼ばないことにする
		Err(e) => { panic!("{:#?}", e);}
	}
}