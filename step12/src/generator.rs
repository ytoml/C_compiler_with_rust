use crate::{exit_eprintln};
use crate::parser::{Node, Nodekind};
use std::rc::Rc;
use std::cell::RefCell;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static ASM: Lazy<Mutex<String>> = Lazy::new(
	|| Mutex::new(
		".intel_syntax noprefix\n.globl main\n".to_string()
	)
);

static CTR_COUNT: Lazy<Mutex<u32>> = Lazy::new(
	|| Mutex::new(0)
);

pub fn gen(node: &Rc<RefCell<Node>>) {
	// 葉にきた、もしくは葉の親のところで左辺値にに何かしらを代入する操作がきた場合の処理
	match (**node).borrow().kind {
		Nodekind::NumNd => {
			// NumNdの時点でunwrapできる
			*ASM.lock().unwrap() += format!("	push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
			return;
		},
		Nodekind::LvarNd => {
			// 葉、かつローカル変数なので、あらかじめ代入した値へのアクセスを行う
			gen_lval(node);
			*ASM.lock().unwrap() += "	pop rax\n"; // gen_lval内で対応する変数のアドレスをスタックにプッシュしているので、popで取れる
			*ASM.lock().unwrap() += "	mov rax, [rax]\n";
			*ASM.lock().unwrap() += "	push rax\n";
			return;
		},
		Nodekind::AssignNd => {
			// 節点、かつアサインゆえ左は左辺値の葉を想定(違えばgen_lval内でエラー)
			gen_lval((**node).borrow().left.as_ref().unwrap());
			gen((**node).borrow().right.as_ref().unwrap());

			// 上記gen2つでスタックに変数の値を格納すべきアドレスと、代入する値(式の評価値)がこの順で積んであるはずなので2回popして代入する
			*ASM.lock().unwrap() += "	pop rdi\n"; 
			*ASM.lock().unwrap() += "	pop rax\n"; 
			*ASM.lock().unwrap() += "	mov [rax], rdi\n";
			*ASM.lock().unwrap() += "	push rdi\n"; // 連続代入可能なように、評価値として代入した値をpushする
			return;
		},
		Nodekind::ReturnNd => {
			// リターンならleftの値を評価してretする。
			gen((**node).borrow().left.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	mov rsp, rbp\n";
			*ASM.lock().unwrap() += "	pop rbp\n";
			*ASM.lock().unwrap() += "	ret\n";
			return;
		},
		Nodekind::IfNd => {
			// PENDING
			*CTR_COUNT.lock().unwrap() += 1;
			let end: String = format!(".LEnd{}", *CTR_COUNT.lock().unwrap());

			gen((**node).borrow().enter.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp 0\n"; // falseは0なので、cmp 0が真ならエンドに飛ぶ
			*ASM.lock().unwrap() += format!("	je {}\n", end).as_str();
			
			gen((**node).borrow().branch.as_ref().unwrap());

			// elseに対応
			if let Some(ptr) = (**node).borrow().enter.as_ref() {
				gen(ptr);
			}

			*ASM.lock().unwrap() += format!(".Lend{}\n", CTR_COUNT.lock().unwrap()).as_str();
			return;
		},
		Nodekind::WhileNd => {
			// PENDING
			*CTR_COUNT.lock().unwrap() += 1;
			let begin: String = format!(".LBegin{}\n", *CTR_COUNT.lock().unwrap());
			let end: String = format!(".LEnd{}\n", *CTR_COUNT.lock().unwrap());

			gen((**node).borrow().enter.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp 0\n"; // falseは0なので、cmp 0が真ならエンドに飛ぶ
			*ASM.lock().unwrap() += format!("	je {}", end).as_str();
			
			gen((**node).borrow().branch.as_ref().unwrap());

			*ASM.lock().unwrap() += format!("	jmp {}", begin).as_str();

			*ASM.lock().unwrap() += end.as_str();
			return;
		},
		Nodekind::ForNd => {
			// PENDING
			*CTR_COUNT.lock().unwrap() += 1;
			let begin: String = format!(".LBegin{}\n", *CTR_COUNT.lock().unwrap());
			let end: String = format!(".LEnd{}\n", *CTR_COUNT.lock().unwrap());

			if let Some(ptr) = (**node).borrow().init.as_ref() {
				gen(ptr);
			}
			*ASM.lock().unwrap() += begin.as_str();
			gen((**node).borrow().enter.as_ref().unwrap());

			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp 0\n"; // falseは0なので、cmp 0が真ならエンドに飛ぶ
			*ASM.lock().unwrap() += format!("	je {}", end).as_str();
			
			gen((**node).borrow().branch.as_ref().unwrap()); // for文内の処理
			gen((**node).borrow().routine.as_ref().unwrap()); // インクリメントなどの処理

			*ASM.lock().unwrap() += format!("	jmp {}", begin).as_str();

			*ASM.lock().unwrap() += end.as_str();
			return;
		}, 
		_ => {}// 他のパターンなら、ここでは何もしない
	} 

	gen((**node).borrow().left.as_ref().unwrap());
	gen((**node).borrow().right.as_ref().unwrap());

	*ASM.lock().unwrap() += "	pop rdi\n";
	*ASM.lock().unwrap() += "	pop rax\n";

	// >, >= についてはオペランド入れ替えのもとsetl, setleを使う
	match (**node).borrow().kind {
		Nodekind::AddNd => {
			*ASM.lock().unwrap() += "	add rax, rdi\n";
		},
		Nodekind::SubNd => {
			*ASM.lock().unwrap() += "	sub rax, rdi\n";
		},
		Nodekind::MulNd => {
			*ASM.lock().unwrap() += "	imul rax, rdi\n";
		},
		Nodekind::DivNd  => {
			*ASM.lock().unwrap() += "	cqo\n";
			*ASM.lock().unwrap() += "	idiv rdi\n";
		},
		Nodekind::EqNd => {
			*ASM.lock().unwrap() += "	cmp rax, rdi\n";
			*ASM.lock().unwrap() += "	sete al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		Nodekind::NEqNd => {
			*ASM.lock().unwrap() += "	cmp rax, rdi\n";
			*ASM.lock().unwrap() += "	setne al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		Nodekind::LThanNd => {
			*ASM.lock().unwrap() += "	cmp rax, rdi\n";
			*ASM.lock().unwrap() += "	setl al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		Nodekind::LEqNd => {
			*ASM.lock().unwrap() += "	cmp rax, rdi\n";
			*ASM.lock().unwrap() += "	setle al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		Nodekind::GThanNd => {
			*ASM.lock().unwrap() += "	cmp rdi, rax\n";
			*ASM.lock().unwrap() += "	setl al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		Nodekind::GEqNd => {
			*ASM.lock().unwrap() += "	cmp rdi, rax\n";
			*ASM.lock().unwrap() += "	setle al\n";
			*ASM.lock().unwrap() += "	movzb rax, al\n";
		},
		_ => {
			// 上記にないNodekindはここに到達する前にreturnしているはず
			exit_eprintln!("不正なNodekindです");
		},
	}

	*ASM.lock().unwrap() += "	push rax\n";

}

// 正しく左辺値を識別して不正な代入("(a+1)=2;"のような)を防ぐためのジェネレータ関数
fn gen_lval(node: &Rc<RefCell<Node>>) {
	if (**node).borrow().kind != Nodekind::LvarNd {
		exit_eprintln!("代入の左辺値が変数ではありません");
	}

	// 変数に対応するアドレスをスタックにプッシュする
	*ASM.lock().unwrap() += "	mov rax, rbp\n";
	*ASM.lock().unwrap() += format!("	sub rax, {}\n", (**node).borrow().offset.as_ref().unwrap()).as_str();
	*ASM.lock().unwrap() += "	push rax\n";
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
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_muldiv() {
		println!("test_{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2*3-4/2".to_string());
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_brackets() {
		let equation = "(1+2)/3-1*20".to_string();
		println!("test_brackets{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_unary() {
		let equation = "(-1+2)*(-1)+(+3)/(+1)".to_string();
		println!("test_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());

	}
	
	#[test]
	fn test_eq() {
		let equation = "(-1+2)*(-1)+(+3)/(+1) == 30 + 1".to_string();
		println!("test_unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_assign_1() {
		let equation = "a = 1; a + 1;".to_string();
		println!("test_assign{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());

	}
	#[test]
	fn test_assign_2() {
		let equation = "local = 1; local_value = local + 1; local_value99 = local_value + 3;".to_string();
		println!("test_assign{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_if() {
		let equation = "
			i = 10;
			if (1) i + 1;
			x = i + 10;
		".to_string();
		println!("test_if{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_while() {
		let equation = "
			i = 10;
			while (i > 1) i = i - 1;
			i;
		".to_string();
		println!("test_while{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());

	}

	#[test]
	fn test_for() {
		let equation = "
			sum = 10;
			for (i = 0; i < 10; i = i + 1) sum += i;
			sum;
		".to_string();
		println!("test_for{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());

	}
}