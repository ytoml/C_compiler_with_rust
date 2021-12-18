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

現在、上記記事の step は全て実装済みで、
- 基本的な単項、二項演算
	- `+=` のような演算代入や前置/後置のインクリメント/デクリメントにも対応
	- `sizeof` にも対応していますが、現在整数型を `int` しかサポートしていないため、 `int` として扱われます。
- int, char 型の変数とそれらへのポインタ(ポインタへのポインタを含む)
	- ポインタ演算に対応しています。例えば `int x = 10; int *y = &x; int *z = y + 2;` とした場合、`z` は `x` の格納されているアドレスから8大きいアドレスを指します。
		- ただし、現在の実装上すべての変数を8の倍数アドレスでアラインメントしているため、int 型の変数 `x` に対して `&x+1` が前の変数のアドレスを指さないことに注意してください。
	- ポインタは全く同じ型どうしの場合のみに引き算ができ、それらのアドレスオフセットが変数いくつ分になるかが評価値となります。
	- スタックにプッシュする値を全て8バイトで処理している関係で、符号拡張などが甘い部分があります。今後修正予定です。
- 配列型の変数と添字によるアクセス
- ローカル変数宣言時の初期化;
- グローバル変数及びその初期化
- 文字列リテラル及び char リテラル
	- utf-8 です
- for, while, if による制御構文
- コンマによる複数文の記述
- 行・ブロックコメント

がサポートされています。  
また、引数6つまでの関数宣言・呼び出しにも対応しています。ただし、引数に式を入れた場合にそれらの式を処理する順番が後ろの引数からの逆順になってしまうという仕様になってしまっており、修正予定です。  
ヘッダファイルの include をサポートしていないため、例えば `printf` のような標準ライブラリを使いたい場合などは、別の C ソースでそれらをラップした関数を定義して gcc 等で x86_64 向けにコンパイルした実行オブジェクトを rscc で改めてコンパイルした元のソースにリンクさせて呼び出す必要があります。(以下の `print_helper`, `showChar`, `printf_wrap` はその例です。)

```C
int fib(int);
int MEMO[100] = {1, 2, 3};
int X[10][20][30];
int x, xx;
int *p = &x;
char c[10][1000] = {"abcd", "str", {'c'}}, d[] = "compiler";

int main() {
	int *p = &X[0][0][0];
	int **pp = &p;
	***X = 10;
	int a = 0;
	int i = 4, x;

	for (int i = 0; i < 10 ; i++) {
		int i = 10;
		a++;
	}
	print_helper(a);
	print_helper(i);

	for(i=0; i < 3; i++) {
		print_helper(MEMO[i]);
		MEMO[i] = 0;
	}
	
	X[0][3][2] = 99;
	print_helper(X[0][2][32]);
	print_helper(sizeof X);

	int X[10][10][10];
	print_helper(sizeof &X);	// (*)int[10][10][10] なので、8 bytes
	print_helper(X);			// addr
	print_helper(X[1]);			// addr + 0x190
	print_helper(&X+1);			// addr + 0xFA0
	X[0][1][1] = 100;
	
	print_helper((x = 19, x = fib(*&(**pp))));
	print_helper(fib(50));

	showChar(c[0][0], c[0][1], c[0][2], c[0][3], 101, 102);
	showChar(d[0], d[1], d[2], d[3], d[4], d[5]);

	char *str = "This is test script";
	showChar(str[13], str[14], str[15], str[16], str[17], str[18]);

	char str2[] = {"This is test script",}, str3[] = "rustcc";
	showChar(str2[13], str2[14], str2[15], str2[16], str2[17], str2[18]);
	showChar(str3[0], str3[1], str3[2], str3[3], str3[4], str3[5]);

	char lf = 10;
	printf_wrap("This is test script for step%d%c", 'a'-69, lf);

	return x;
}

/**
 * メモ化再帰による fibonacci
 */
int fib(int N) {
	if (N <= 2) return 1;
	if (MEMO[N-1]) return MEMO[N-1];
	return MEMO[N-1] = fib(N-1) + fib(N-2);
}
```
上記のプログラムの出力は、リンクさせる関数内でどう表示するかにもよりますが、例えば以下のようになります。ただし、最後の行は exit status を表示しています。
```
I got 10 as argument.
I got 4 as argument.
I got 1 as argument.
I got 2 as argument.
I got 3 as argument.
I got 99 as argument.
I got 24000 as argument.
I got 8 as argument.
I got 274903129360 as argument.
I got 274903129760 as argument.
I got 274903133360 as argument.
I got 55 as argument.
I got 3996334433 as argument.
showChar called, message is "abcdef"
showChar called, message is "compil"
showChar called, message is "script"
showChar called, message is "script"
showChar called, message is "rustcc"
This is test script for step28
55
```