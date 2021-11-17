
const UNSUPPORTED_REG_SIZE: &str = "unsupported register size";


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
macro_rules! operate {
	($operator:expr) => {
		*ASM.try_lock().unwrap() += format!("\t{}\n", $operator).as_str()
	};
	
	($operator:expr, $operand1:expr) => {
		*ASM.try_lock().unwrap() += format!("\t{} {}\n", $operator, $operand1).as_str()
	};
	
	($operator:expr, $operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\t{} {}, {}\n", $operator, $operand1, $operand2).as_str()
	};
}

#[macro_export]
macro_rules! mov_to {
	($size:expr, $operand1:expr, $operand2:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		*ASM.try_lock().unwrap() += format!("\tmov {} [{}], {}\n", _word, $operand1, $operand2).as_str()
	};

	($size:expr, $operand1:expr, $operand2:expr, $offset:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		*ASM.try_lock().unwrap() += format!("\tmov {} [{}-{}], {}\n", _word, $operand1, $offset, $operand2).as_str()
	};
}

#[macro_export]
macro_rules! mov_from {
	($size:expr, $operand1:expr, $operand2:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		*ASM.try_lock().unwrap() += format!("\tmov {}, {} [{}]\n", $operand1, _word, $operand2).as_str()
	};

	($size:expr, $operand1:expr, $operand2:expr, $offset:expr) => {
		use crate::asm::word_ptr;
		let _word = word_ptr($size);
		*ASM.try_lock().unwrap() += format!("\tmov {}, {} [{}-{}]\n", $operand1, _word,$operand2, $offset).as_str()
	};
}

#[macro_export]
macro_rules! mov {
	($operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov {}, {}\n", $operand1, $operand2).as_str()
	};
}

#[macro_export]
macro_rules! lea {
	($operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\tlea {}, [{}]\n", $operand1, $operand2).as_str()
	};

	($operand1:expr, $operand2:expr, $offset:expr) => {
		*ASM.try_lock().unwrap() += format!("\tlea {}, [{}-{}]\n", $operand1, $operand2, $offset).as_str()
	};
}

#[macro_export]
macro_rules! asm_write {
	($fmt: expr) => {
		*ASM.try_lock().unwrap() += format!($fmt).as_str()
	};

	($fmt: expr, $($arg: tt)*) =>{
		*ASM.try_lock().unwrap() += format!($fmt, $($arg)*).as_str()
	};
}