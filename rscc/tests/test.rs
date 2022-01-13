use std::fs::{remove_file, File};
use std::io::{self, Write};
use std::process::{Command, ExitStatus};

use rscc::compile_src;

const SRC: &str = "tests/utils/test.c";
const ASM: &str = "tests/tmp.s";
const RUN: &str = "tests/utils/run.sh";

macro_rules! cprintln {
	($fmt:expr, $color:expr) => {
		println!(concat!("\x1b[{}m", $fmt, "\x1b[m"), $color);
	};

	($fmt:expr, $color:expr, $($args:tt)*) => {
		println!(concat!("\x1b[{}m", $fmt, "\x1b[m"), $color, $($args)*);
	};
}

#[test]
pub fn rscc_test() {
    let asm = compile_src(SRC);
    assert!(asm.is_ok());
    cprintln!("compile succeeded!", 36);

    assert!(output_asm(asm.unwrap()).is_ok());
    cprintln!("assembly successfully created!", 36);

    let status = exec_asm();
    assert!(status.is_ok());
    assert!(status.unwrap().success());
    assert!(remove_file(ASM).is_ok());
    cprintln!("test succeeded!", 36);
}

fn output_asm(asm: String) -> io::Result<()> {
    let mut f = File::create(ASM)?;
    f.write_all(asm.as_bytes())?;
    Ok(())
}

fn exec_asm() -> io::Result<ExitStatus> {
    let output = Command::new(RUN).arg(ASM).output()?;
    cprintln!("output follows {}", 32, ">".repeat(40));
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    cprintln!("output end {}", 32, "<".repeat(44));
    Ok(output.status)
}
