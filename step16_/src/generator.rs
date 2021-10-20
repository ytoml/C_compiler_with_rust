use crate::{
	node::{Node, Nodekind},
	exit_eprintln,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

use once_cell::sync::Lazy;

pub static ASM: Lazy<Mutex<String>> = Lazy::new(
	|| Mutex::new(
		".intel_syntax noprefix\n.globl main\n".to_string()
	)
);

static CTR_COUNT: Lazy<Mutex<u32>> = Lazy::new(
	|| Mutex::new(0)
);

static ARGS_REGISTERS: Lazy<Mutex<Vec<&str>>> = Lazy::new(|| Mutex::new(vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"]));

// CTR_COUNT にアクセスして分岐ラベルのための値を得つつインクリメントする
fn get_count() -> u32 {
	*CTR_COUNT.lock().unwrap() += 1;
	*CTR_COUNT.lock().unwrap()
}

pub fn gen(node: &Rc<RefCell<Node>>) {
	// 葉にきた、もしくは葉の親のところで左辺値にに何かしらを代入する操作がきた場合の処理
	match (**node).borrow().kind {
		Nodekind::FuncDecNd => {
			*ASM.lock().unwrap() += format!("{}:\n", (**node).borrow().name.as_ref().unwrap()).as_str();
		
			// プロローグ(変数の格納領域の確保)
			*ASM.lock().unwrap() += "	push rbp\n";
			*ASM.lock().unwrap() += "	mov rbp, rsp\n";
			let pull = (**node).borrow().max_offset.unwrap();
			if pull > 0 {
				*ASM.lock().unwrap() += format!("	sub rsp, {}\n", (**node).borrow().max_offset.unwrap()).as_str() ;
			}

			// 受け取った引数の挿入: 現在は6つの引数までなのでレジスタから値を持ってくる
			if (*node).borrow().args.len() > 6 {exit_eprintln!("現在7つ以上の引数はサポートされていません。");}
			for (ix, arg) in (&(*node).borrow().args).iter().enumerate() {
				*ASM.lock().unwrap() += "	mov rax, rbp\n";
				*ASM.lock().unwrap() += format!("	sub rax, {}\n", (*(*arg.as_ref().unwrap())).borrow().offset.as_ref().unwrap()).as_str();
				*ASM.lock().unwrap() += format!("	mov [rax], {}\n", ARGS_REGISTERS.lock().unwrap()[ix]).as_str();
			}
			
			// 関数内の文の処理
			let s = (*node).borrow().stmts.as_ref().unwrap().len();
			for (ix, stmt_) in (*node).borrow().stmts.as_ref().unwrap().iter().enumerate() {
				gen(stmt_);
				if ix != s - 1 {*ASM.lock().unwrap() += "	pop rax\n";}
			}

			// 上の stmts の処理で return が書かれることになっているので、エピローグなどはここに書く必要はない
			return;
		}
		Nodekind::NumNd => {
			// NumNdの時点でunwrapできる
			*ASM.lock().unwrap() += format!("	push {}\n", (**node).borrow().val.as_ref().unwrap()).as_str();
			return;
		}
		Nodekind::LogAndNd => {
			let c = get_count();
			let f_anchor: String = format!(".LLogic.False{}", c);
			let e_anchor: String = format!(".LLogic.End{}", c);

			// && の左側 (short circuit であることに注意)
			gen((**node).borrow().left.as_ref().unwrap());
			{
				let mut _asm = ASM.lock().unwrap();
				*_asm += "	pop rax\n";
				*_asm += "	cmp rax, 0\n";
				*_asm += format!("	je {}\n", f_anchor).as_str(); // 0 なら false ゆえ残りの式の評価はせずに飛ぶ 
			}

			// && の右側
			gen((**node).borrow().right.as_ref().unwrap());
			let mut _asm = ASM.lock().unwrap();
			*_asm += "	pop rax\n";
			*_asm += "	cmp rax, 0\n";
			*_asm += format!("	je {}\n", f_anchor).as_str();

			// true の場合、 rax に 1 をセットして end
			*_asm += "	mov rax, 1\n";
			*_asm += format!("	jmp {}\n", e_anchor).as_str();

			*_asm += format!("{}:\n", f_anchor).as_str();
			*_asm += "	mov rax, 0\n";

			*_asm += format!("{}:\n", e_anchor).as_str();
			// *_asm += "	cdqe\n"; // rax でなく eax を使う場合は、上位の bit をクリアする必要がある(0 をきちんと false にするため)
			*_asm += "	push rax\n";

			return;
		}
		Nodekind::LogOrNd => {
			let c = get_count();
			let t_anchor: String = format!(".LLogic.False{}", c);
			let e_anchor: String = format!(".LLogic.End{}", c);

			// && の左側 (short circuit であることに注意)
			gen((**node).borrow().left.as_ref().unwrap());
			{
				let mut _asm = ASM.lock().unwrap();
				*_asm += "	pop rax\n";
				*_asm += "	cmp rax, 0\n";
				*_asm += format!("	jne {}\n", t_anchor).as_str(); // 0 なら false ゆえ残りの式の評価はせずに飛ぶ 
			}

			// && の右側
			gen((**node).borrow().right.as_ref().unwrap());
			let mut _asm = ASM.lock().unwrap();
			*_asm += "	pop rax\n";
			*_asm += "	cmp rax, 0\n";
			*_asm += format!("	jne {}\n", t_anchor).as_str();

			// false の場合、 rax に 0 をセットして end
			*_asm += "	mov rax, 1\n";
			*_asm += format!("	jmp {}\n", e_anchor).as_str();

			*_asm += format!("{}:\n", t_anchor).as_str();
			*_asm += "	mov rax, 1\n";

			*_asm += format!("{}:\n", e_anchor).as_str();
			// *_asm += "	cdqe\n"; // rax でなく eax を使う場合は、上位の bit をクリアする必要がある(0 をきちんと false にするため)
			*_asm += "	push rax\n";

			return;
		}
		Nodekind::LogNotNd => {
			gen((**node).borrow().left.as_ref().unwrap());
			let mut _asm = ASM.lock().unwrap();
			*_asm += "	pop rax\n";

			// rax が 0 なら 1, そうでないなら 0 にすれば良い
			*_asm += "	cmp rax, 0\n";
			*_asm += "	sete al\n";
			*_asm += "	movzb rax, al\n";
			*_asm += "	push rax\n";

			return;
		}
		Nodekind::BitNotNd => {
			gen((**node).borrow().left.as_ref().unwrap());
			let mut _asm = ASM.lock().unwrap();
			*_asm += "	pop rax\n";
			*_asm += "	not rax\n";
			*_asm += "	push rax\n";

			return;
		}
		Nodekind::LvarNd => {
			// 葉、かつローカル変数なので、あらかじめ代入した値へのアクセスを行う
			gen_lval(node);
			*ASM.lock().unwrap() += "	pop rax\n"; // gen_lval内で対応する変数のアドレスをスタックにプッシュしているので、popで取れる
			*ASM.lock().unwrap() += "	mov rax, [rax]\n";
			*ASM.lock().unwrap() += "	push rax\n";
			return;
		}
		Nodekind::DerefNd => {
			// gen内で *var の var のアドレスをスタックにプッシュしたことになる
			gen((**node).borrow().left.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n"; 
			*ASM.lock().unwrap() += "	mov rax, [rax]\n";
			*ASM.lock().unwrap() += "	push rax\n";
			return;
		}
		Nodekind::AddrNd => {
			// gen_lval内で対応する変数のアドレスをスタックにプッシュしているので、そのままでOK
			// 生成規則上は Deref も Addr と同様に複数つけられる(&&var)ことになっているが、本当はそんなことないので、ここで gen_lval を使うことで担保する
			gen_lval((**node).borrow().left.as_ref().unwrap());
			return;
		}
		Nodekind::FuncNd => {
			// 引数をレジスタに格納する処理
			push_args(&(**node).borrow().args);
			
			*ASM.lock().unwrap() += "	mov rax, rsp\n";
			*ASM.lock().unwrap() += format!("	and rsp, ~0x10\n").as_str(); // 16の倍数に align
			*ASM.lock().unwrap() += "	sub rsp, 8\n";
			*ASM.lock().unwrap() += "	push rax\n";
			// この時点で ARGS_REGISTERS に記載の6つのレジスタには引数が入っている必要がある
			*ASM.lock().unwrap() += format!("	call {}\n", (**node).borrow().name.as_ref().unwrap()).as_str();
			*ASM.lock().unwrap() += "	pop rsp\n";
			*ASM.lock().unwrap() += "	push rax\n";
			return;
		}
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
		}
		Nodekind::CommaNd => {
			// 式の評価値として1つ目の結果は捨てる
			gen((**node).borrow().left.as_ref().unwrap());
			{
				let mut _asm = ASM.lock().unwrap();
				*_asm += "	pop rax\n"; 
			}
			// 2つ目の式の評価値はそのまま使うので、popなしでOK
			gen((**node).borrow().right.as_ref().unwrap());
			return;
		}
		Nodekind::ReturnNd => {
			// リターンならleftの値を評価してretする。
			gen((**node).borrow().left.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	mov rsp, rbp\n";
			*ASM.lock().unwrap() += "	pop rbp\n";
			*ASM.lock().unwrap() += "	ret\n";
			return;
		}
		Nodekind::IfNd => {
			// PENDING
			let c: u32 = get_count();
			let end: String = format!(".LEnd{}", c);

			// 条件文の処理
			gen((**node).borrow().enter.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp rax, 0\n"; 

			// elseがある場合は微妙にjmp命令の位置が異なることに注意
			if let Some(ptr) = (**node).borrow().els.as_ref() {
				let els: String = format!(".LElse{}", c);

				// falseは0なので、cmp rax, 0が真ならelseに飛ぶ
				*ASM.lock().unwrap() += format!("	je {}\n", els).as_str();
				gen((**node).borrow().branch.as_ref().unwrap()); // if(true)の場合の処理
				*ASM.lock().unwrap() += format!("	jmp {}\n", end).as_str(); // elseを飛ばしてendへ

				// elseの後ろの処理
				*ASM.lock().unwrap() += format!("{}:\n", els).as_str();
				gen(ptr);
				*ASM.lock().unwrap() += "	pop rax\n"; // 今のコードでは各stmtはpush raxを最後にすることになっているので、popが必要

			} else {
				// elseがない場合の処理
				*ASM.lock().unwrap() += format!("	je {}\n", end).as_str();
				gen((**node).borrow().branch.as_ref().unwrap());
				*ASM.lock().unwrap() += "	pop rax\n"; // 今のコードでは各stmtはpush raxを最後にすることになっているので、popが必要
			}

			// stmtでgenした後にはpopが呼ばれるはずであり、分岐後いきなりpopから始まるのはおかしい(し、そのpopは使われない)
			// ブロック文やwhile文も単なる num; などと同じようにstmt自体が(使われない)戻り値を持つものだと思えば良い
			*ASM.lock().unwrap() += format!("{}:\n", end).as_str();
			*ASM.lock().unwrap() += "	push 0\n"; 

			return;
		}
		Nodekind::WhileNd => {
			let c: u32 = get_count();
			let begin: String = format!(".LBegin{}", c);
			let end: String = format!(".LEnd{}", c);

			*ASM.lock().unwrap() += format!("{}:\n", begin).as_str();
			gen((**node).borrow().enter.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp rax, 0\n"; // falseは0なので、cmp rax, 0が真ならエンドに飛ぶ
			*ASM.lock().unwrap() += format!("	je {}\n", end).as_str();
			
			gen((**node).borrow().branch.as_ref().unwrap());
			*ASM.lock().unwrap() += "	pop rax\n"; // 今のコードでは各stmtはpush raxを最後にすることになっているので、popが必要

			*ASM.lock().unwrap() += format!("	jmp {}\n", begin).as_str();

			// if文と同じ理由でpushが必要
			*ASM.lock().unwrap() += format!("{}:\n", end).as_str();
			*ASM.lock().unwrap() += "	push 0\n"; 

			return;
		}
		Nodekind::ForNd => {
			let c: u32 = get_count();
			let begin: String = format!(".LBegin{}", c);
			let end: String = format!(".LEnd{}", c);

			if let Some(ptr) = (**node).borrow().init.as_ref() {
				gen(ptr);
			}

			*ASM.lock().unwrap() += format!("{}:\n", begin).as_str();
			gen((**node).borrow().enter.as_ref().unwrap());

			*ASM.lock().unwrap() += "	pop rax\n";
			*ASM.lock().unwrap() += "	cmp rax, 0\n"; // falseは0なので、cmp rax, 0が真ならエンドに飛ぶ
			*ASM.lock().unwrap() += format!("	je {}\n", end).as_str();
			
			gen((**node).borrow().branch.as_ref().unwrap()); // for文内の処理
			*ASM.lock().unwrap() += "	pop rax\n"; // 今のコードでは各stmtはpush raxを最後にすることになっているので、popが必要

			
			gen((**node).borrow().routine.as_ref().unwrap()); // インクリメントなどの処理

			*ASM.lock().unwrap() += format!("	jmp {}\n", begin).as_str();

			// if文と同じ理由でpushが必要
			*ASM.lock().unwrap() += format!("{}:\n", end).as_str();
			*ASM.lock().unwrap() += "	push 0\n"; 

			return;
		} 
		Nodekind::BlockNd => {

			for child in &(**node).borrow().children {
				// parserのコード的にNoneなchildはありえないはずであるため、直にunwrapする
				gen(child.as_ref().unwrap());
				*ASM.lock().unwrap() += "	pop rax\n"; // 今のコードでは各stmtはpush raxを最後にすることになっているので、popが必要
			}
			
			// このBlock自体がstmt扱いであり、このgenがreturnした先でもpop raxが生成されるはず
			// これもif文と同じくpush 0をしておく
			*ASM.lock().unwrap() += "	push 0\n"; 

			return;
		}
		_ => {}// 他のパターンなら、ここでは何もしない
	} 

	gen((**node).borrow().left.as_ref().unwrap());
	gen((**node).borrow().right.as_ref().unwrap());

	let mut _asm = ASM.lock().unwrap();
	if [Nodekind::LShiftNd, Nodekind::RShiftNd].contains(&(**node).borrow().kind) {
		*_asm += "	pop rcx\n";
	} else {
		*_asm += "	pop rdi\n";
	}
	*_asm += "	pop rax\n";

	// >, >= についてはオペランド入れ替えのもとsetl, setleを使う
	match (**node).borrow().kind {
		Nodekind::AddNd => {
			*_asm += "	add rax, rdi\n";
		}
		Nodekind::SubNd => {
			*_asm += "	sub rax, rdi\n";
		}
		Nodekind::MulNd => {
			*_asm += "	imul rax, rdi\n";
		}
		Nodekind::DivNd  => {
			*_asm += "	cqo\n"; // rax -> rdx:rax に拡張(ただの 0 fill)
			*_asm += "	idiv rdi\n"; // rdi で割る: rax が商で rdx が剰余になる
		}
		Nodekind::ModNd  => {
			*_asm += "	cqo\n";
			*_asm += "	idiv rdi\n";
			*_asm += "	push rdx\n";
			return;
		}
		Nodekind::LShiftNd => {
			*_asm += "	sal rax, cl\n";
		}
		Nodekind::RShiftNd => {
			*_asm += "	sar rax, cl\n";
		}
		Nodekind::BitAndNd => {
			*_asm += "	and rax, rdi\n";
		}
		Nodekind::BitOrNd => {
			*_asm += "	or rax, rdi\n";
		}
		Nodekind::BitXorNd => {
			*_asm += "	xor rax, rdi\n";
		}
		Nodekind::EqNd => {
			*_asm += "	cmp rax, rdi\n";
			*_asm += "	sete al\n";
			*_asm += "	movzb rax, al\n";
		}
		Nodekind::NEqNd => {
			*_asm += "	cmp rax, rdi\n";
			*_asm += "	setne al\n";
			*_asm += "	movzb rax, al\n";
		}
		Nodekind::LThanNd => {
			*_asm += "	cmp rax, rdi\n";
			*_asm += "	setl al\n";
			*_asm += "	movzb rax, al\n";
		}
		Nodekind::LEqNd => {
			*_asm += "	cmp rax, rdi\n";
			*_asm += "	setle al\n";
			*_asm += "	movzb rax, al\n";
		}
		Nodekind::GThanNd => {
			*_asm += "	cmp rdi, rax\n";
			*_asm += "	setl al\n";
			*_asm += "	movzb rax, al\n";
		}
		Nodekind::GEqNd => {
			*_asm += "	cmp rdi, rax\n";
			*_asm += "	setle al\n";
			*_asm += "	movzb rax, al\n";
		}
		_ => {
			// 上記にないNodekindはここに到達する前にreturnしているはず
			exit_eprintln!("不正なNodekindです");
		}
	}

	*_asm += "	push rax\n";

}

// 正しく左辺値を識別して不正な代入("(a+1)=2;"のような)を防ぐためのジェネレータ関数
fn gen_lval(node: &Rc<RefCell<Node>>) {
	match (**node).borrow().kind {
		Nodekind::LvarNd => {
			// 変数に対応するアドレスをスタックにプッシュする
			*ASM.lock().unwrap() += "	mov rax, rbp\n";
			*ASM.lock().unwrap() += format!("	sub rax, {}\n", (**node).borrow().offset.as_ref().unwrap()).as_str();
			*ASM.lock().unwrap() += "	push rax\n";
		}
		Nodekind::DerefNd => {
			// &* は単に打ち消せば良く、node を無視して gen(node->left) する
			gen((**node).borrow().left.as_ref().unwrap());
		}
		_ => {exit_eprintln!("左辺値が変数ではありません。");}
	}
}

// 関数呼び出し時の引数の処理を行う関数
fn push_args(args: &Vec<Option<Rc<RefCell<Node>>>>) {
	let argc =  args.len();
	if argc > 6 {exit_eprintln!("現在7つ以上の引数はサポートされていません。");}

	// 計算時に rdi などを使う場合があるので、引数はまずはスタックに全て push したままにしておく
	// おそらく、逆順にしておいた方がスタックに引数を積みたくなった場合に都合が良い
	for i in (0..argc).rev() {
		gen(&(args[i]).as_ref().unwrap());
	}

	for i in 0..argc {
		*ASM.lock().unwrap() += format!("	pop {}\n", (*ARGS_REGISTERS.lock().unwrap())[i]).as_str();
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::tokenizer::*;
	use crate::parser::*;
	use crate::parser::tests::parse_stmts;

	static REP:usize = 80;


	#[test]
	fn addsub() {
		println!("addsub{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2+3-1".to_string());
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn muldiv() {
		println!("muldiv{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize("1+2*3-4/2+3%2".to_string());
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn brackets() {
		let equation = "(1+2)/3-1*20".to_string();
		println!("brackets{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn unary() {
		let equation = "(-1+2)*(-1)+(+3)/(+1)".to_string();
		println!("unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn shift() {
		let equation = "
			200 % 3 << 4 + 8 >> 8
		".to_string();
		println!("unary{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}
	
	#[test]
	fn eq() {
		let equation = "(-1+2)*(-1)+(+3)/(+1) == 30 + 1".to_string();
		println!("eq{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_ptr = expr(&mut token_ptr);
		gen(&node_ptr);

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn assign1() {
		let equation = "a = 1; a + 1;".to_string();
		println!("assign1{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn assign2() {
		let equation = "local = 1; local_value = local + 1; local_value99 = local_value + 3;".to_string();
		println!("assign2{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn bitops() {
		let equation = "
			2 + (3 + 5) * 6;
			1 ^ 2 | 2 != 3 / 2;
			1 + -1 ^ 2;
			3 ^ 2 & 1 | 2 & 9;
			x = 10;
			y = &x;
			3 ^ 2 & *y | 2 & &x;
			~x ^ ~*y | 2;
		".to_string();
		println!("bitops{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);

			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn logops() {
		let equation = "
			x = 10;
			y = 20;
			z = 20;
			q = !x && !!y - z || 0;
		".to_string();
		println!("logops{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn comma() {
		let equation = "
			x = 10, y = 10, z = 10;
		".to_string();
		println!("comma{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn if_() {
		let equation = "
			i = 10;
			if (1) i + 1;
			x = i + 10;
		".to_string();
		println!("if{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn while_() {
		let equation = "
			i = 10;
			while (i > 1) i = i - 1;
			i;
		".to_string();
		println!("while{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn for_() {
		let equation = "
			sum = 10;
			for (i = 0; i < 10; i = i + 1) sum = sum + i;
			return sum;
		".to_string();
		println!("for{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}
	
	#[test]
	fn block() {
		let equation = "
			sum = 10;
			sum2 = 20;
			for (i = 0; i < 10; i = i + 1) {
				sum = sum + i;
				sum2 = sum2 + i;
			}
			return sum;
			return;
		".to_string();
		println!("block{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}
	
	#[test]
	fn func() {
		let equation = "
			call_fprint();
			i = get(1);
			j = get(2, 3, 4);
			k = get(i+j, (i=3), k);
			return i + j;
		".to_string();
		println!("func{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn addr_deref() {
		let equation = "
			x = 3;
			y = 5;
			z = &y + 8;
			return *z;;
		".to_string();
		println!("addr_deref{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn addr_deref2() {
		let equation = "
			x = 3;
			y = &x;
			z = &y;
			return *&**z;
		".to_string();
		println!("addr_deref2{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = parse_stmts(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
			*ASM.lock().unwrap() += "	pop rax\n";
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn funcdec() {
		let equation = "
			func(x, y) {
				return x * (y + 1);
			}
			sum(i, j) {
				return i + j;
			}
			main() {
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
				return func(i, sum);
			}
		".to_string();
		println!("funcdec{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
		}

		println!("{}", ASM.lock().unwrap());
	}

	#[test]
	fn recursion() {
		let equation = "
			fib(n) {
				return fib(n-1)+fib(n-2);
			}
			main() {
				return fib(10);
			}
		".to_string();
		println!("recursion{}", "-".to_string().repeat(REP));
		let mut token_ptr = tokenize(equation);
		let node_heads = program(&mut token_ptr);
		for node_ptr in node_heads {
			gen(&node_ptr);
		}

		println!("{}", ASM.lock().unwrap());
	}
}