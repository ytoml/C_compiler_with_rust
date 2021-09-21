# C_compiler_with_rust

CコンパイラをRustで作ることで、コンパイラとRustの勉強を一気にしてしまおうという試みです。
Rui Ueyamaさんの
[低レイヤを知りたい人のためのCコンパイラ作成入門](https://www.sigbus.info/compilerbook)を読みながら作成していきます。


自分の開発環境の都合上、Dockerコンテナ上での動作を想定しています。
最終的には、自分のローカルにあるCのソースを自作コンパイラでコンパイルしてx86_64向けバイナリを出力し、
```
./exec.sh {dockerイメージ名} {実行ファイル名}
```
で実行できるようにする予定です。

現在、上記記事のstep13まで実装しており、基本的な計算と型を考慮しない変数の使用、for, while, ifの3種類の制御構文がサポートされています。

```
sum = 55;
k = 1;
for (i = 0; i < 11; i = i + 1) {
	sum = sum + 1;
	if ( k > 0 ) sum = sum + 1;
	k = k * (-1);
}

if (sum != 72) {
	sum = 0;
	sum = sum + k;
	return sum;
}

i = 21;
k = 0;

if ((sum + 1) == 72) {
	i = 100;
} else {
	i = 1000;
}

while ( i > -1 ) {
	k = k + 1;
	{
		{}
		i =  i + k / 2;
	}
	i = i-k;
}
if (i >= 0) {
	return i;
}

10;

return 100;
return i;
return -1;
```