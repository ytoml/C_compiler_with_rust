use rscc::compile_src;

#[test]
pub fn rscc_test() {
    match compile_src("tests/test.txt") {
		Ok(asm) => {
			println!("test succeeded!");
			println!("{}", asm);
		}
		Err(err) => { panic!("test failed: {:#?}", err); }
	}
}