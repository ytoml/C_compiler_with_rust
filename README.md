# C_compiler_with_rust

C コンパイラを Rust で作ることで、コンパイラと Rust の勉強を一気にしてしまおうという試みです。
Rui Ueyama さんの
[低レイヤを知りたい人のためのCコンパイラ作成入門](https://www.sigbus.info/compilerbook)を読みながら作成していきます。


自分の開発環境の都合上、Docker コンテナ上での動作を想定しています。
最終的には、自分のローカルにあるCのソースを自作コンパイラでコンパイルして x86_64 向けバイナリを出力し、
```
./exec.sh {dockerイメージ名} {実行ファイル名}
```
で実行できるようにする予定です。

現在、上記記事のstep16まで実装しており、基本的な計算と型を考慮しない変数とその参照(ポインタの演算などはサポートしていません)、for, while, ifの3種類の制御構文がサポートされています。
また、引数6つまでの関数宣言・呼び出しにも対応しています。ただし、型の宣言や変数宣言のみの文には対応していません。
`printf` などを使いたい場合も、別の C ソースで関数の中身を定義して gcc 等で x86_64 向けにコンパイルした実行オブジェクトとリンクさせて呼び出す必要があります。(以下の `print_helper`, `print_something`)

```
fib(N) {
	if (N <= 2) return 1;
	return fib(N-1) + fib(N-2);
}

calc (a, b, c, d, e, f) {
	return a*b + c - d + e/f;
}

main () {
	x = 100;
	p = &x;
	y = 20;
	
	for (i = 0; i < 20; i = i+2) {
		print_helper(*p = *p-i);
	}

	if (x < 0) return x;
	else {
		x = (x + y) / 2;
	}

	print_something();
	
	while (i > 0) {
		print_helper(i = i - 1);
	}

	pp = &p;

	print_something();

	print_helper(z = fib(*&(**pp)));
	print_helper(z + calc(1, 2, 3, 4, 5, 6));
}
```