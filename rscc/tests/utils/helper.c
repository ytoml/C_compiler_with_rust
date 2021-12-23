#include <stdio.h>

int print_helper(long long x) {
	printf("I got %lld as argument.\n", x);
	return 0;
}

int printf_wrap(const char *fmt, char x, char c) {
	printf(fmt, x, c);
	return 0;
}

int showChar(char c1, char c2, char c3, char c4, char c5, char c6) {
	printf("showChar called, message is \"%c%c%c%c%c%c\"\n", c1, c2, c3, c4, c5, c6);
	return 0;
}