use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static CODE: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec!["".to_string()]));

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
	($fmt:expr) => {
		eprint!($fmt);
		std::process::exit(1);
	};

	// 第二引数以降がある場合 
	($fmt:expr, $($arg:tt)*) =>{
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
	($fmt:expr) => {
		eprint!(concat!($fmt, "\n"));
		std::process::exit(1);
	};

	// 第二引数以降がある場合 
	($fmt:expr, $($arg:tt)*) =>(
		eprint!(concat!($fmt, "\n"),$($arg)*);
		std::process::exit(1);
	);
}

// エラー位置を報告するバージョンを作りたかったが、今の実装でやるのが難しそうなので保留
#[macro_export]
macro_rules!  error_at{
	($fmt: expr, $num: expr, $($arg:tt)*) => {
		let space = " ".to_string().repeat(*$num);
		eprint!(concat!(space, $fmt, "\n"),$($arg)*);
		std::process::exit(1);
	};
}