int fib(int);
int MEMO[10*10] = {1, 2, 3};
int X[10][20][30];
int x, xx = 1 + 9 + (1 + 3)/ 4;
int *p = &x;
char c[10][990+2*5] = {"abcd", "str", {'c'}}, d[] = "compiler";

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

	int z, *q=&z;
	print_helper(q==&z+10-10);

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