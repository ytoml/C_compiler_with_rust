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

現在、上記記事の step27 まで実装しており、
- 基本的な単項、二項演算
	- `+=` のような演算代入や前置/後置のインクリメント/デクリメントにも対応
	- `sizeof` にも対応していますが、現在整数型を `int` しかサポートしていないため、 `int` として扱われます。
- int, char 型の変数とそれらへのポインタ(ポインタへのポインタを含む)
	- ポインタ演算に対応しています。例えば `int x = 10; int *y = &x; int *z = y + 2;` とした場合、`z` は `x` の格納されているアドレスから8大きいアドレスを指します。
		- ただし、現在の実装上すべての変数を8の倍数アドレスでアラインメントしているため、int 型の変数 `x` に対して `&x+1` が前の変数のアドレスを指さないことに注意してください。
	- ポインタは全く同じ型どうしの場合のみに引き算ができ、それらのアドレスオフセットが変数いくつ分になるかが評価値となります。
	- スタックにプッシュする値を全て8バイトで処理している関係で、符号拡張などが甘い部分があります。今後修正予定です。
- 配列型の変数と添字によるアクセス
- グローバル変数
- 文字列リテラル
- for, while, if による制御構文
- コンマによる複数文の記述
- 行・ブロックコメント

がサポートされています。  
また、引数6つまでの関数宣言・呼び出しにも対応しています。ただし、引数に式を入れた場合にそれらの式を処理する順番が後ろの引数からの逆順になってしまうという仕様になってしまっており、修正予定です。  
ヘッダファイルの include をサポートしていないため、例えば `printf` のような標準ライブラリを使いたい場合などは、別の C ソースでそれらをラップした関数を定義して gcc 等で x86_64 向けにコンパイルした実行オブジェクトを rscc で改めてコンパイルした元のソースにリンクさせて呼び出す必要があります。(以下の `print_helper`, `showChar`, `printf_wrap` はその例です。)

```C
int fib(int);
int MEMO[100];
int X[10][20][30];
char c[10];

int main() {
	int i, x;
	int *p = &X[0][0][0];
	int **pp = &p;
	***X = 10;

	for(i=0; i < 100; i++) {
		MEMO[i] = 0;
	}
	
	X[0][3][2] = 99;
	print_helper(X[0][2][32]);
	print_helper(sizeof X);

	int X[10][10][10];
	print_helper(sizeof &X);	// (*)int[10][10][10] -> 8 bytes
	print_helper(X);			// addr
	print_helper(X[1]);			// addr + 0x190
	print_helper(&X+1);			// addr + 0xFA0
	X[0][1][1] = 100;
	
	print_helper((x = 19, x = fib(*&(**pp))));
	print_helper(fib(50));

	char *str = "This is test script";
	showChar(str[13], str[14], str[15], str[16], str[17], str[18]);

	char lf = 10;
	printf_wrap("This is test script for step%d%c", 25, lf);

	return x;
}

/**
 * Recursive fibonacci with memoization.
 */
int fib(int N) {
	if (N <= 2) return 1;
	if (MEMO[N-1]) return MEMO[N-1];
	return MEMO[N-1] = fib(N-1) + fib(N-2);
}
```