use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static CODES: Lazy<Mutex<Vec<Vec<String>>>> = Lazy::new(|| Mutex::new(vec![]));
pub static FILE_NAMES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

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



// エラー位置を報告し、exit_eprintln! する関数
const RED: usize = 31;
const LIGHTBLUE: usize = 36;
pub fn error_at(msg: &str, file_num: usize, line_num: usize, line_offset: usize) -> ! {
	let file_name = &FILE_NAMES.lock().unwrap()[file_num];
	let code_line = &CODES.lock().unwrap()[file_num][line_num];
	let all_space = code_line.chars().map(|c| if c == '\t' {'\t'} else {' '}).collect::<String>();
	let space = &all_space[..line_offset];
	eprintln!("\x1b[{}m{}: {}\x1b[m", LIGHTBLUE, file_name, line_num);
	eprint!("{}", code_line); // code_line には \n が含まれるので eprint! を使う
	exit_eprintln!("{}\x1b[{}m^\x1b[m {}", space, RED, msg);
}

