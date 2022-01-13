use crate::{
    asm::{cast, get_ctrl_count, get_func_count, reg_ax, word_ptr, ARGS_REGISTERS, ASMCODE},
    asm_write, error_with_node, exit_eprintln, lea, mov, mov_from, mov_from_glb, mov_glb_addr,
    mov_op, mov_to, movsx,
    node::{NodeRef, Nodekind},
    operate,
    parser::ORDERED_LITERALS,
    typecell::Type,
};
use std::rc::Rc;

pub fn generate(trees: Vec<NodeRef>) {
    load_literals();
    for tree in trees {
        gen_expr(&tree);
    }
}

fn load_literals() {
    let literals_access = ORDERED_LITERALS.try_lock().unwrap();
    if literals_access.is_empty() {
        return;
    }

    asm_write!("\t.section .rodata"); // read-only data
    for (body, name) in literals_access.iter() {
        asm_write!("{}:", name);
        asm_write!("\t.string \"{}\"", body);
    }
}

/// 各計算結果が rax に保持された形になるようなコードを出力
fn gen_expr(node: &NodeRef) {
    let kind = node.borrow().kind;
    match kind {
        Nodekind::GlobalNd => {
            let node = node.borrow();
            let name = node.name.clone().unwrap();
            if let Some(_) = &node.func_typ {
                // プロトタイプ宣言は無視して OK
                if node.stmts.is_none() {
                    return;
                }
                let c = get_func_count();

                asm_write!("\t.text");
                asm_write!("\t.globl {}", name);
                asm_write!("\t.type {}, @function", name);
                asm_write!("{}:", name);
                asm_write!(".LFB{}:", c); // function begin label

                // プロローグ(変数の格納領域の確保)
                operate!("push", "rbp");
                mov!("rbp", "rsp");
                let pull = node.max_offset.unwrap();
                if pull > 0 {
                    operate!("sub", "rsp", pull);
                }

                // 受け取った引数の挿入: 現在は6つの引数までなのでレジスタから値を持ってくる
                if node.args.len() > 6 {
                    exit_eprintln!("現在7つ以上の引数はサポートされていません。");
                }
                for (ix, arg) in (&node.args).iter().enumerate() {
                    let offset = *arg.borrow().offset.as_ref().unwrap();
                    let size = arg.borrow().typ.as_ref().unwrap().bytes();
                    let arg_reg = ARGS_REGISTERS.try_lock().unwrap().get(&size).unwrap()[ix];

                    mov_to!(size, "rbp", arg_reg, offset);
                }

                // 関数内の文の処理
                for stmt in node.stmts.as_ref().unwrap().iter() {
                    gen_expr(stmt);
                }
                asm_write!(".LFE{}:", c); // function begin label
                asm_write!("\t.size {}, .-{}", name, name);
            // 上の stmts の処理で return が書かれることになっているので、エピローグなどはここに書く必要はない
            } else {
                // 現在はグローバル変数の初期化はサポートしないため、常に .bss で指定
                let typ = node.typ.clone().unwrap();
                let bytes = typ.bytes();
                let is_initialized = node.init_data.len() > 0;
                asm_write!("\t.globl {}", name);
                if is_initialized {
                    let is_ptr = if typ.is_array() {
                        typ.get_base_cell().is_pointer()
                    } else {
                        typ.is_pointer()
                    };
                    if is_ptr {
                        asm_write!("\t.section .data.rel.local");
                    }
                    asm_write!("\t.data");
                } else {
                    asm_write!("\t.bss");
                }
                asm_write!("\t.type {}, @object", name);
                asm_write!("\t.size {}, {}", name, bytes);
                asm_write!("{}:", name);
                if node.init_data.len() > 0 {
                    for data in &node.init_data {
                        if let Some(label) = &data.label {
                            if data.size != 8 {
                                panic!("something wrong with initializing data size");
                            }
                            if data.val == 0 {
                                asm_write!(".quad {}", label);
                            } else if data.val > 0 {
                                asm_write!(".quad {}+{}", label, data.val);
                            } else {
                                asm_write!(".quad {}{}", label, data.val);
                            }
                        } else {
                            if data.val == 0 {
                                asm_write!("\t.zero {}", data.size);
                            } else {
                                match data.size {
                                    1 => {
                                        asm_write!("\t.byte {}", data.val);
                                    }
                                    2 => {
                                        asm_write!("\t.value {}", data.val);
                                    }
                                    4 => {
                                        asm_write!("\t.long {}", data.val);
                                    }
                                    8 => {
                                        asm_write!("\t.quad {}", data.val);
                                    }
                                    _ => {
                                        panic!("something wrong with initializing data size");
                                    }
                                }
                            }
                        }
                    }
                } else {
                    asm_write!("\t.zero {}", bytes);
                }
            }
            return;
        }
        Nodekind::NumNd => {
            mov!("rax", node.borrow().val.unwrap());
            return;
        }
        Nodekind::LogAndNd => {
            let c = get_ctrl_count();
            let f_anchor: String = format!(".LLogic.False{}", c);
            let e_anchor: String = format!(".LLogic.End{}", c);

            // && の左側 (short circuit であることに注意)
            gen_expr(node.borrow().left.as_ref().unwrap());
            operate!("cmp", "rax", 0);
            operate!("je", f_anchor); // 0 なら false ゆえ残りの式の評価はせずに飛ぶ

            // && の右側
            gen_expr(node.borrow().right.as_ref().unwrap());
            operate!("cmp", "rax", 0);
            operate!("je", f_anchor);

            // true の場合、 rax に 1 をセットして end
            mov!("rax", 1);
            operate!("jmp", e_anchor);

            asm_write!("{}:", f_anchor);
            mov!("rax", 0);

            asm_write!("{}:", e_anchor);
            // operate!("cdqe"); // rax でなく eax を使う場合は、上位の bit をクリアする必要がある(0 をきちんと false にするため)
            return;
        }
        Nodekind::LogOrNd => {
            let c = get_ctrl_count();
            let t_anchor: String = format!(".LLogic.False{}", c);
            let e_anchor: String = format!(".LLogic.End{}", c);

            // && の左側 (short circuit であることに注意)
            gen_expr(node.borrow().left.as_ref().unwrap());
            operate!("cmp", "rax", 0);
            operate!("jne", t_anchor); // 0 なら false ゆえ残りの式の評価はせずに飛ぶ

            // && の右側
            gen_expr(node.borrow().right.as_ref().unwrap());
            operate!("cmp", "rax", 0);
            operate!("jne", t_anchor);

            // false の場合、 rax に 0 をセットして end
            mov!("rax", 1);
            operate!("jmp", e_anchor);

            asm_write!("{}:", t_anchor);
            mov!("rax", 1);

            asm_write!("{}:", e_anchor);
            // operate!("cdqe"); // rax でなく eax を使う場合は、上位の bit をクリアする必要がある(0 をきちんと false にするため)
            return;
        }
        Nodekind::LogNotNd => {
            gen_expr(node.borrow().left.as_ref().unwrap());

            // rax が 0 なら 1, そうでないなら 0 にすれば良い
            operate!("cmp", "rax", 0);
            operate!("sete", "al");
            operate!("movzb", "rax", "al");
            return;
        }
        Nodekind::BitNotNd => {
            gen_expr(node.borrow().left.as_ref().unwrap());
            operate!("not", "rax");
            return;
        }
        Nodekind::LvarNd => {
            // 葉、かつローカル変数なので、あらかじめ代入した値へのアクセスを行う
            // 配列のみ、それ単体でアドレスとして解釈されるため gen_addr の結果をそのまま使うことにしてスルー
            let typ = node.borrow().typ.clone();
            if typ.clone().unwrap().typ != Type::Array {
                // movsx などで eax を使うことに注意
                let bytes = typ.unwrap().bytes();
                let ax = if bytes < 4 { "eax" } else { reg_ax(bytes) };

                if node.borrow().is_local {
                    let offset = node.borrow().offset.unwrap();
                    mov_from!(bytes, ax, "rbp", offset);
                } else {
                    let name = node.borrow().name.clone().unwrap();
                    mov_from_glb!(bytes, ax, name);
                }

                // rax で push するために、 eax ならば符号拡張が必要(現在は4と8しかサポートしていないためこうなる)
                if bytes == 4 {
                    operate!("cdqe");
                }
            } else {
                gen_addr(node);
            }

            return;
        }
        Nodekind::DerefNd => {
            // gen_expr内で *expr の expr のアドレスをスタックにプッシュしたことになる
            // 配列との整合をとるために *& の場合に打ち消す必要がある
            let left = Rc::clone(node.borrow().left.as_ref().unwrap());
            if left.borrow().kind == Nodekind::AddrNd {
                gen_expr(left.borrow().left.as_ref().unwrap());
            } else {
                // 参照を外した後でも配列なのであれば、アドレスが指す値を評価せずそのまま使用する
                gen_expr(&left);
                if node.borrow().typ.as_ref().unwrap().typ != Type::Array {
                    let left_typ = left.borrow().typ.clone().unwrap();
                    let bytes = if left_typ.typ == Type::Array {
                        8
                    } else {
                        left_typ.bytes()
                    };
                    mov_from!(bytes, "rax", "rax");
                }
            }
            return;
        }
        Nodekind::AddrNd => {
            gen_addr(node.borrow().left.as_ref().unwrap());
            return;
        }
        Nodekind::FunCallNd => {
            // 引数をレジスタに格納する処理
            push_args(&node.borrow().args);

            mov!("rax", "rsp");
            operate!("and", "rsp", "~0x0f"); // 16の倍数に align
            operate!("sub", "rsp", 8);
            operate!("push", "rax");

            // この時点で ARGS_REGISTERS に記載の6つのレジスタには引数が入っている必要がある
            mov!("rax", 0); // 可変長引数をとる際、浮動小数点の数を al に入れる必要があるが、今は浮動小数点がサポートされていないため単に0を入れる
            operate!("call", node.borrow().name.as_ref().unwrap());
            operate!("pop", "rsp");
            return;
        }
        Nodekind::AssignNd => {
            // 節点、かつアサインゆえ左は左辺値の葉を想定(違えばgen_addr内でエラー)
            gen_addr(node.borrow().left.as_ref().unwrap());
            operate!("push", "rax");
            gen_expr(node.borrow().right.as_ref().unwrap()); // この時点で rax に代入値、スタックトップに変数のアドレス

            // 上記gen_expr2つでスタックに変数の値を格納すべきアドレスと、代入する値(式の評価値)がこの順で積んであるはずなので2回popして代入する
            let typ = node.borrow().typ.clone().unwrap();
            let bytes = if typ.typ == Type::Array {
                8
            } else {
                typ.bytes()
            };
            operate!("pop", "rdi");
            mov_to!(bytes, "rdi", reg_ax(bytes));
            return;
        }
        Nodekind::CastNd => {
            let node = node.borrow();
            let left = node.left.as_ref().unwrap();
            let from = left.borrow().typ.as_ref().unwrap().typ;
            let to = node.typ.as_ref().unwrap().typ;
            gen_expr(left);
            cast(from, to);
            return;
        }
        Nodekind::CommaNd => {
            // 式の評価値として1つ目の結果は捨て、2つめの評価値のみが rax に残る
            gen_expr(node.borrow().left.as_ref().unwrap());
            gen_expr(node.borrow().right.as_ref().unwrap());
            return;
        }
        Nodekind::ReturnNd => {
            // リターンならleftの値を評価してretする。
            gen_expr(node.borrow().left.as_ref().unwrap());
            mov!("rsp", "rbp");
            operate!("pop", "rbp");
            operate!("ret");
            return;
        }
        Nodekind::IfNd => {
            let c: u32 = get_ctrl_count();
            let end: String = format!(".LEnd{}", c);

            // 条件文の処理
            gen_expr(node.borrow().enter.as_ref().unwrap());
            operate!("cmp", "rax", 0);

            // elseがある場合は微妙にjmp命令の位置が異なることに注意
            if let Some(ptr) = node.borrow().els.as_ref() {
                let els: String = format!(".LElse{}", c);

                // falseは0なので、cmp rax, 0が真ならelseに飛ぶ
                operate!("je", els);
                gen_expr(node.borrow().branch.as_ref().unwrap()); // if(true)の場合の処理
                operate!("jmp", end); // elseを飛ばしてendへ

                // elseの後ろの処理
                asm_write!("{}:", els);
                gen_expr(ptr);
            } else {
                // elseがない場合の処理
                operate!("je", end);
                gen_expr(node.borrow().branch.as_ref().unwrap());
            }
            asm_write!("{}:", end);
            return;
        }
        Nodekind::WhileNd => {
            let c: u32 = get_ctrl_count();
            let begin: String = format!(".LBegin{}", c);
            let end: String = format!(".LEnd{}", c);

            asm_write!("{}:", begin);

            gen_expr(node.borrow().enter.as_ref().unwrap());
            operate!("cmp", "rax", 0); // falseは0なので、cmp rax, 0が真ならエンドに飛ぶ
            operate!("je", end);

            gen_expr(node.borrow().branch.as_ref().unwrap());
            operate!("jmp", begin);

            asm_write!("{}:", end);
            return;
        }
        Nodekind::ForNd => {
            let c: u32 = get_ctrl_count();
            let begin: String = format!(".LBegin{}", c);
            let end: String = format!(".LEnd{}", c);

            if let Some(init) = node.borrow().init.as_ref() {
                gen_expr(init);
            }

            asm_write!("{}:", begin);

            if let Some(enter) = &node.borrow().enter {
                gen_expr(enter);
                operate!("cmp", "rax", 0); // falseは0なので、cmp rax, 0が真ならエンドに飛ぶ
                operate!("je", end);
            }

            gen_expr(node.borrow().branch.as_ref().unwrap()); // for文内の処理

            if let Some(routine) = &node.borrow().routine {
                gen_expr(routine); // インクリメントなどの処理
            }
            operate!("jmp", begin);

            asm_write!("{}:", end);
            return;
        }
        Nodekind::BlockNd => {
            for child in &node.borrow().children {
                gen_expr(child);
            }
            return;
        }
        Nodekind::ZeroClrNd => {
            // これは特殊な Node で、現時点では left に LvarNd が繋がっているパターンしかあり得ない
            let left = Rc::clone(node.borrow().left.as_ref().unwrap());
            let offset = left.borrow().offset.unwrap();
            let bytes = left.borrow().typ.clone().unwrap().bytes();
            zero_clear(offset, bytes);
            return;
        }
        Nodekind::NopNd => {
            return;
        }
        _ => {} // 他のパターンなら、ここでは何もしない
    }

    let left = Rc::clone(node.borrow().left.as_ref().unwrap());
    let right = Rc::clone(node.borrow().right.as_ref().unwrap());
    gen_expr(&left);
    operate!("push", "rax");
    gen_expr(&right);

    // long や long long などが実装されるまではポインタなら8バイト、そうでなければ4バイトのレジスタを使うことにする
    let (ax, di, dx, cq) = if left.borrow().typ.as_ref().unwrap().ptr_end.is_some() {
        ("rax", "rdi", "rdx", "cqo")
    } else {
        ("eax", "edi", "edx", "cdq")
    };

    if [Nodekind::LShiftNd, Nodekind::RShiftNd].contains(&node.borrow().kind) {
        mov!("rcx", "rax");
    } else {
        mov!("rdi", "rax");
    }
    operate!("pop", "rax");

    // >, >= についてはオペランド入れ替えのもとsetl, setleを使う
    match node.borrow().kind {
        Nodekind::AddNd => {
            operate!("add", ax, di);
        }
        Nodekind::SubNd => {
            operate!("sub", ax, di);
        }
        Nodekind::MulNd => {
            operate!("imul", ax, di);
        }
        Nodekind::DivNd => {
            operate!(cq); // rax -> rdx:rax に拡張(ただの 0 fill)
            operate!("idiv", di); // rdi で割る: rax が商で rdx が剰余になる
        }
        Nodekind::ModNd => {
            operate!(cq);
            operate!("idiv", di);
            mov!(ax, dx);
        }
        Nodekind::LShiftNd => {
            operate!("sal", ax, "cl");
        }
        Nodekind::RShiftNd => {
            operate!("sar", ax, "cl");
        }
        Nodekind::BitAndNd => {
            operate!("and", ax, di);
        }
        Nodekind::BitOrNd => {
            operate!("or", ax, di);
        }
        Nodekind::BitXorNd => {
            operate!("xor", ax, di);
        }
        Nodekind::EqNd => {
            operate!("cmp", ax, di);
            operate!("sete", "al");
            operate!("movzb", "rax", "al");
        }
        Nodekind::NEqNd => {
            operate!("cmp", ax, di);
            operate!("setne", "al");
            operate!("movzb", "rax", "al");
        }
        Nodekind::LThanNd => {
            operate!("cmp", ax, di);
            operate!("setl", "al");
            operate!("movzb", "rax", "al");
        }
        Nodekind::LEqNd => {
            operate!("cmp", ax, di);
            operate!("setle", "al");
            operate!("movzb", "rax", "al");
        }
        _ => {
            // 上記にないNodekindはここに到達する前にreturnしているはず
            error_with_node!("不正な Nodekind です。", &*node.borrow());
        }
    }
}

/// アドレスを生成し、 rax に保存する
fn gen_addr(node: &NodeRef) {
    let node = node.borrow();
    let kind = node.kind;
    match kind {
        Nodekind::LvarNd => {
            if node.is_local {
                // 変数に対応するアドレスをスタックにプッシュする
                let offset = node.offset.unwrap();
                lea!("rax", "rbp", offset);
            } else {
                let name = node.name.clone().unwrap();
                mov_glb_addr!("rax", name);
            }
        }
        Nodekind::DerefNd => {
            // *expr: exprで計算されたアドレスを返したいので直で gen_expr する(例えば&*のような書き方だと打ち消される)
            gen_expr(node.left.as_ref().unwrap());
        }
        _ => {
            error_with_node!("左辺値が変数ではありません。", &*node);
        }
    }
}

/// 関数呼び出し時の引数の処理を行う
fn push_args(args: &Vec<NodeRef>) {
    let argc = args.len();
    if argc > 6 {
        exit_eprintln!("現在7つ以上の引数はサポートされていません。");
    }

    // 計算時に rdi などを使う場合があるので、引数はまずはスタックに全て push したままにしておく
    // おそらく、逆順にしておいた方がスタックに引数を積みたくなった場合に都合が良い
    if argc != 0 {
        operate!("sub", "rsp", argc * 8);
        for i in 0..argc {
            gen_expr(&args[i]);
            if i == 0 {
                asm_write!("\tmov QWORD PTR[rsp], rax");
            } else {
                asm_write!("\tmov QWORD PTR[rsp+{}], rax", i * 8);
            }
        }
    }

    for i in 0..argc {
        let typ = args[i].borrow().typ.clone().unwrap();
        let bytes = if typ.typ == Type::Array {
            8
        } else {
            typ.bytes()
        };
        let arg_reg = ARGS_REGISTERS.try_lock().unwrap().get(&bytes).unwrap()[i];
        let arg_reg_r = ARGS_REGISTERS.try_lock().unwrap().get(&8).unwrap()[i];
        let ax = reg_ax(bytes);
        operate!("pop", "rax");
        // rax で push するために符号拡張する
        if bytes == 4 {
            operate!("cdqe");
        } else if bytes < 4 {
            movsx!("rax", "al");
        }
        if bytes < 8 {
            mov!(arg_reg_r, 0);
        }
        mov!(arg_reg, ax);
    }
}

/// rbp - offset から rbp - offset + bytes までゼロクリアを行う
fn zero_clear(mut offset: usize, mut bytes: usize) {
    if bytes >= 128 {
        lea!("rdi", "rbp", offset);
        mov!("rax", 0);
        mov!("rcx", bytes / 8);
        operate!("rep", "stosq");
        mov!("eax", 0);
        offset -= (bytes / 8) * 8;
    } else {
        for _ in 0..bytes / 8 {
            mov_to!(8, "rbp", 0, offset);
            offset -= 8;
        }
    }
    bytes %= 8;
    if bytes / 4 == 1 {
        mov_to!(4, "rbp", 0, offset);
        offset -= 4;
    }
    bytes %= 4;
    if bytes / 2 == 1 {
        mov_to!(2, "rbp", 0, offset);
        offset -= 2;
    }
    bytes %= 2;
    if bytes == 1 {
        mov_to!(1, "rbp", 0, offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::{FILE_NAMES, SRC};
    use crate::parser::{expr, parse, tests::parse_stmts};
    use crate::tokenizer::tokenize;

    fn test_init(src: &str) {
        let mut src_: Vec<String> = src.split("\n").map(|s| s.to_string() + "\n").collect();
        FILE_NAMES.try_lock().unwrap().push("test".to_string());
        let mut code = vec!["".to_string()];
        code.append(&mut src_);
        SRC.try_lock().unwrap().push(code);
    }

    #[test]
    fn addsub() {
        let src: &str = "
			1+2+3-1
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn muldiv() {
        let src: &str = "
			1+2*3-4/2+3%2
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn brackets() {
        let src: &str = "
			(1+2)/3-1*20
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn unary() {
        let src: &str = "
			(-1+2)*(-1)+(+3)/(+1)
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn shift() {
        let src: &str = "
			200 % 3 << 4 + 8 >> 8
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn eq() {
        let src: &str = "
			(-1+2)*(-1)+(+3)/(+1) == 30 + 1
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_ptr = expr(&mut token_ptr);
        gen_expr(&node_ptr);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn assign1() {
        let src: &str = "
			int a;
			a = 1; a + 1;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn assign2() {
        let src: &str = "
			int local, local_value, local_value99;
			local = 1; local_value = local + 1; local_value99 = local_value + 3;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn bitops() {
        let src: &str = "
			int x, y;
			2 + (3 + 5) * 6;
			1 ^ 2 | 2 != 3 / 2;
			1 + -1 ^ 2;
			3 ^ 2 & 1 | 2 & 9;
			x = 10;
			y = &x;
			3 ^ 2 & *y | 2 & &x;
			~x ^ ~*y | 2;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn logops() {
        let src: &str = "
			int x, y, z, q;
			x = 10;
			y = 20;
			z = 20;
			q = !x && !!y - z || 0;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn comma() {
        let src: &str = "
			int x, y, z;
			x = 10, y = 10, z = 10;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn if_() {
        let src: &str = "
			int i;
			i = 10;
			if (1) i + 1;
			x = i + 10;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn while_() {
        let src: &str = "
			int i;
			i = 10;
			while (i > 1) i = i - 1;
			i;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn for_() {
        let src: &str = "
			int sum, i;
			sum = 10;
			for (i = 0; i < 10; i = i + 1) sum = sum + i;
			return sum;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn block() {
        let src: &str = "
			int sum, sum2, i;
			sum = 10;
			sum2 = 20;
			for (i = 0; i < 10; i = i + 1) {
				sum = sum + i;
				sum2 = sum2 + i;
			}
			return sum;
			return;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn func() {
        let src: &str = "
			int i, j, k;
			call_fprint();
			i = get(1);
			j = get(2, 3, 4);
			k = get(i+j, (i=3), k);
			return i + j;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn addr_deref() {
        let src: &str = "
			int x, y, z;
			x = 3;
			y = 5;
			z = &y + 8;
			return *z;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn addr_deref2() {
        let src: &str = "
			int x, y, z;
			x = 3;
			y = &x;
			z = &y;
			return *&**z;
		";
        test_init(src);

        let mut token_ptr = tokenize(0);
        let node_heads = parse_stmts(&mut token_ptr);
        for node_ptr in node_heads {
            gen_expr(&node_ptr);
        }
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn funcdec() {
        let src: &str = "
			int func(int x, int y) {
				return x * (y + 1);
			}
			int sum(int i, int j) {
				return i + j;
			}
			int main() {
				int i, sum;
				i = 0;
				sum = 0;
				for (; i < 10; i=i+1) {
					sum = sum + i;
				}
				return func(i, sum);
			}
		";
        test_init(src);

        let head = tokenize(0);
        let trees = parse(head);
        generate(trees);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn recursion() {
        let src: &str = "
			int fib(int n) {
				return fib(n-1)+fib(n-2);
			}
			int main() {
				return fib(10);
			}
		";
        test_init(src);

        let head = tokenize(0);
        let trees = parse(head);
        generate(trees);
        println!("{}", ASMCODE.try_lock().unwrap());
    }

    #[test]
    fn zero_clear_() {
        let src: &str = "
			int main() {
				char X[][9][33] = {{1}, 3};
				char Y[127] = {0};
				int Z[15] = {0};
				return 0;
			}
		";
        test_init(src);

        let head = tokenize(0);
        let trees = parse(head);
        generate(trees);
        println!("{}", ASMCODE.try_lock().unwrap());
    }
}
