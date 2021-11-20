use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

const UNSUPPORTED_REG_SIZE: &str = "unsupported register size";

pub static ASMCODE: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(
	"\t.intel_syntax noprefix\n\t.text\n.LText0:\n".to_string()
));

pub static ARGS_REGISTERS: Lazy<Mutex<HashMap<usize, Vec<&str>>>> = Lazy::new(|| {
	let mut map = HashMap::new();
	let _ = map.insert(4, vec!["edi", "esi", "edx", "rcx", "r8d", "r9d"]);
	let _ = map.insert(8, vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"]);
	Mutex::new(map)
});

static CTRL_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
static FUNC_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

// CTRL_COUNT にアクセスして分岐ラベルのための値を得つつインクリメントする
pub fn get_ctrl_count() -> u32 {
	let mut count = CTRL_COUNT.try_lock().unwrap();
	let c = *count;
	*count += 1;
	c
}

pub fn get_func_count() -> u32 {
	let mut count = FUNC_COUNT.try_lock().unwrap();
	let c = *count;
	*count += 1;
	c
}

pub fn reg_ax(size: usize) -> &'static str {
	match size {
		4 => { "eax" }
		8 => { "rax" }
		_ => { panic!("{}", UNSUPPORTED_REG_SIZE); }
	}
}

pub fn reg_di(size: usize) -> &'static str {
	match size {
		4 => { "edi" }
		8 => { "rdi" }
		_ => { panic!("{}", UNSUPPORTED_REG_SIZE); }
	}
}

pub fn word_ptr(size: usize) -> &'static str {
	match size {
		4 => { "DWORD PTR" }
		8 => { "QWORD PTR" }
		_ => { panic!("{}", UNSUPPORTED_REG_SIZE); }
	}
}

#[macro_export]
macro_rules! asm_write {
	($fmt: expr) => {
		*ASMCODE.try_lock().unwrap() += format!(concat!($fmt, "\n")).as_str()
	};

	($fmt: expr, $($arg: tt)*) =>{
		*ASMCODE.try_lock().unwrap() += format!(concat!($fmt, "\n"), $($arg)*).as_str()
	};
}

#[macro_export]
macro_rules! operate {
	($operator:expr) => {
		asm_write!("\t{}", $operator)
	};
	
	($operator:expr, $operand1:expr) => {
		asm_write!("\t{} {}", $operator, $operand1)
	};
	
	($operator:expr, $operand1:expr, $operand2:expr) => {
		asm_write!("\t{} {}, {}", $operator, $operand1, $operand2)
	};
}

#[macro_export]
macro_rules! mov_to {
	($size:expr, $operand1:expr, $operand2:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {} [{}], {}", _word, $operand1, $operand2)
	};

	($size:expr, $operand1:expr, $operand2:expr, $offset:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {} [{}-{}], {}", _word, $operand1, $offset, $operand2)
	};
}

#[macro_export]
macro_rules! mov_from {
	($size:expr, $operand1:expr, $operand2:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {}, {} [{}]", $operand1, _word, $operand2)
	};

	($size:expr, $operand1:expr, $operand2:expr, $offset:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {}, {} [{}-{}]", $operand1, _word,$operand2, $offset)
	};
}

#[macro_export]
macro_rules! mov_glb_addr {
	($operand:expr, $name:expr) => {
		asm_write!("\tlea {}, {}[rip]", $operand, $name)
	};
}

#[macro_export]
macro_rules! mov_from_glb {
	($size:expr, $operand:expr, $name:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {}, {} {}[rip]", $operand, _word, $name)
	};
}

#[macro_export]
macro_rules! mov_to_glb {
	($size:expr, $operand:expr, $name:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		asm_write!("\tmov {} {}[rip], {}", _word, $name, $operand)
	};
}

#[macro_export]
macro_rules! mov {
	($operand1:expr, $operand2:expr) => {
		asm_write!("\tmov {}, {}", $operand1, $operand2)
	};
}

#[macro_export]
macro_rules! lea {
	($operand1:expr, $operand2:expr) => {
		asm_write!("\tlea {}, [{}]", $operand1, $operand2)
	};

	($operand1:expr, $operand2:expr, $offset:expr) => {
		asm_write!("\tlea {}, [{}-{}]", $operand1, $operand2, $offset)
	};
}