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
step16_ というディレクトリではサポートする演算の拡張を行っており、論理演算やインクリメント、演算+代入などもサポートしました。
また、引数6つまでの関数宣言・呼び出しにも対応しています。ただし、型の宣言や変数宣言のみの文には対応していません。
`printf` などを使いたい場合も、別の C ソースで関数の中身を定義して gcc 等で x86_64 向けにコンパイルした実行オブジェクトとリンクさせて呼び出す必要があります。(以下の `print_helper`, `print_something`)

```
func(x, y) {
	print_helper(x+y);
	return x + y;
}

fib(N) {
	if (N <= 2) return 1;
	return fib(N-1) + fib(N-2);
}

main() {
	i = 0;
	j = 0;
	k = 1;
	sum = 0;
	for (; i < 10; i+=i+1, j++) {
		sum++;
	}
	print_helper(j);
	print_helper(k);
	while (j > 0, 0) {
		j /= 2;
		k <<= 1;
	}
	if (1 && !(k/2)) k--;
	else k = -1;

	func(x=1, (y=1, z=~1));

	x = 15 & 10;
	p = &x;
	pp = &p;
	print_helper(z = fib(*&(**pp)));

	return k;
}
```