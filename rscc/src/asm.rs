
const UNSUPPORTED_REG_SIZE: &str = "unsupported register size";


pub fn reg_ax(size: usize) -> &'static str {
	match size {
		4 => { "eax" }
		8 => { "rax" }
		_ => { panic!("{}", UNSUPPORTED_REG_SIZE); }
	}
}

pub fn reg_dx(size: usize) -> &'static str {
	match size {
		4 => { "eax" }
		8 => { "rax" }
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
	($operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov [{}], {}\n", $operand1, $operand2).as_str()
	};

	($operand1:expr, $operand2:expr, $offset:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov [{} - {}], {}\n", $operand1, $offset, $operand2).as_str()
	};
}

#[macro_export]
macro_rules! mov_from {
	($operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov {}, [{}]\n", $operand1, $operand2).as_str()
	};

	($operand1:expr, $operand2:expr, $offset:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov {}, [{} - {}]\n", $operand1, $operand2, $offset).as_str()
	};
}

#[macro_export]
macro_rules! mov {
	($operand1:expr, $operand2:expr) => {
		*ASM.try_lock().unwrap() += format!("\tmov {}, {}\n", $operand1, $operand2).as_str()
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