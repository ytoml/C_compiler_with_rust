use crate::{exit_eprintln};
use crate::parser::{Node, Nodekind};
use std::rc::Rc;
use std::cell::RefCell;

pub fn gen(node: &Rc<RefCell<Node>>, asm: &mut String) {
	// 葉にきた、もしくは葉の親のところで左辺値にに何かしらを代入する操作がきた場合の処理
	match (**node).borrow().kind {
		Nodekind::ND_NUM => {
			// ND_NUMの時点でunwrapできる
			*asm += format!("	push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
			return;
		},
		Nodekind::ND_LVAR => {
			// 葉、かつローカル変数なので、あらかじめ代入した値へのアクセスを行う
			gen_lval(node, asm);
			*asm += "	pop rax\n"; // gen_lval内で対応する変数のアドレスをスタックにプッシュしているので、popで取れる
			*asm += "	mov rax, [rax]\n";
			*asm += "	push rax\n";
			return;
		},
		Nodekind::ND_ASSIGN => {
			// 節点、かつアサインゆえ左は左辺値の葉を想定(違えばgen_lval内でエラー)
			gen_lval((**node).borrow().left.as_ref().unwrap(), asm);
			gen((**node).borrow().right.as_ref().unwrap(), asm);

			// 上記gen2つでスタックに変数の値を格納すべきアドレスと、代入する値(式の評価値)がこの順で積んであるはずなので2回popして代入する
			*asm += "	pop rdi\n"; 
			*asm += "	pop rax\n"; 
			*asm += "	mov [rax], rdi\n";
			*asm += "	push rdi\n"; // 連続代入可能なように、評価値として代入した値をpushする
			return;
		},
		_ => {}// 他のパターンなら、ここでは何もしない
		
	} 

	gen((**node).borrow().left.as_ref().unwrap(), asm);
	gen((**node).borrow().right.as_ref().unwrap(), asm);

	*asm += "	pop rdi\n";
	*asm += "	pop rax\n";

	// >, >= についてはオペランド入れ替えのもとsetl, setleを使う
	match (**node).borrow().kind {
		Nodekind::ND_ADD => {
			*asm += "	add rax, rdi\n";
		},
		Nodekind::ND_SUB => {
			*asm += "	sub rax, rdi\n";
		},
		Nodekind::ND_MUL => {
			*asm += "	imul rax, rdi\n";
		},
		Nodekind::ND_DIV  => {
			*asm += "	cqo\n";
			*asm += "	idiv rdi\n";
		},
		Nodekind::ND_EQ => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	sete al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_NE => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setne al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_LT => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setl al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_LE => {
			*asm += "	cmp rax, rdi\n";
			*asm += "	setle al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_GT => {
			*asm += "	cmp rdi, rax\n";
			*asm += "	setl al\n";
			*asm += "	movzb rax, al\n";
		},
		Nodekind::ND_GE => {
			*asm += "	cmp rdi, rax\n";
			*asm += "	setle al\n";
			*asm += "	movzb rax, al\n";
		},
		_ => {
			// 上記にないNodekindはここに到達する前にreturnしているはず
			exit_eprintln!("不正なNodekindです");
		},
	}

	*asm += "	push rax\n";

}

// 正しく左辺値を識別して不正な代入("(a+1)=2;"のような)を防ぐためのジェネレータ関数
fn gen_lval(node: &Rc<RefCell<Node>>, asm: &mut String) {
	if (**node).borrow().kind != Nodekind::ND_LVAR {
		exit_eprintln!("代入の左辺値が変数ではありません");
	}

	// 変数に対応するアドレスをスタックにプッシュする
	*asm += "	mov rax, rbp\n";
	*asm += format!("	sub rax, {}\n", (**node).borrow().offset.as_ref().unwrap()).as_str();
	*asm += "	push rax\n";
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tokenizer::*;
	use crate::parser::*;

	static REP:usize = 80;


	#[test]
	fn test_addsub() {
		println!("test_{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2+3-1".to_string());
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_muldiv() {
		println!("test_{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2*3-4/2".to_string());
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_brackets() {
		let equation = "(1+2)/3-1*20".to_string();
		println!("test_brackets{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_unary() {
		let equation = "(-1+2)*(-1)+(+3)/(+1)".to_string();
		println!("test_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}
	
	#[test]
	fn test_eq() {
		let equation = "(-1+2)*(-1)+(+3)/(+1) == 30 + 1".to_string();
		println!("test_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		let mut asm = "".to_string();
		gen(&node_ptr, &mut asm);

		println!("{}", asm);

	}

	#[test]
	fn test_assign_1() {
		let equation = "a = 1; a + 1;".to_string();
		println!("test_assign{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		let mut asm = "".to_string();
		for node_ptr in node_heads {
			gen(&node_ptr, &mut asm);

			asm += "	pop rax\n";
		}

		println!("{}", asm);

	}
	#[test]
	fn test_assign_2() {
		let equation = "local = 1; local_value = local + 1; local_value99 = local_value + 3;".to_string();
		println!("test_assign{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		let mut asm = "".to_string();
		for node_ptr in node_heads {
			gen(&node_ptr, &mut asm);

			asm += "	pop rax\n";
		}

		println!("{}", asm);

	}
}