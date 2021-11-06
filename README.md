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

現在、上記記事のstep20まで実装しており、
- 基本的な単項、二項演算
	- `+=` のような演算代入や前置/後置のインクリメント/デクリメントにも対応
	- `sizeof` にも対応していますが、現在整数型を `int` しかサポートしていないため、 `int` として扱われます。
- int 型の変数とそれらへのポインタ(ポインタへのポインタを含む)
	- ポインタ演算に対応しています。例えば `int x = 10; int *y = &x; int *z = y + 2;` とした場合、`z` は `x` の格納されているアドレスから8大きいアドレスを指します。
		- ただし、現在の実装上すべての変数を8の倍数アドレスでアラインメントしているため、int 型の変数 `x` に対して `&x+1` が前の変数のアドレスを指さないことに注意してください。
	- 現時点では宣言と初期化を同時に行うことができず、`int x; x = 10;` のように書く必要があります。
	- ポインタは全く同じ型どうしの場合のみに引き算ができ、それらのアドレスオフセットが変数いくつ分になるかが評価値となります。
- for, while, if による制御構文
- コンマによる複数文の記述
がサポートされています。  
また、引数6つまでの関数宣言・呼び出しにも対応しています。ただし、引数に式を入れた場合にそれらの式を処理する順番が後ろの引数からの逆順になってしまうという仕様になってしまっており、修正予定です。  
ヘッダファイルの include をサポートしていないため、例えば `printf` のような標準ライブラリを使いたい場合などは、別の C ソースでそれらをラップした関数を定義して gcc 等で x86_64 向けにコンパイルした実行オブジェクトを rscc で改めてコンパイルした元のソースにリンクさせて呼び出す必要があります。(以下の `print_helper`, `print_something` はその例です。)

```
int func(int x, int y) {
	print_helper(x+y);
	return x + y;
}

int fib(int N) {
	if (N <= 2) return 1;
	return fib(N-1) + fib(N-2);
}

int main() {
	int i; i = 0;
	int j; j = 0;
	int k; k = 1;
	int sum; sum = 0;
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

	int x, y, z;
	func(x=1, (y=1, z=~1));

	x = 15 & 10;
	x = (++x) + y;
	int *p; p = &x; 
	int **pp; pp = &p;
	*p += 9;
	print_helper(z = fib(*&(**pp)));
	print_helper(*&*&*&**&*pp);
	print_helper(sizeof (x+y));
	print_helper(sizeof ++x);
	print_helper(sizeof &x + x);
	print_helper(sizeof(int**));
	print_helper(sizeof(x && x));
	print_helper(sizeof(*p));

	return k;
}
```