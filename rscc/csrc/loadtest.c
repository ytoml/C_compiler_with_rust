#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <ctype.h>

enum{BUF_SIZE = 16};

int main(int args, char *argv[]){
	int fd,cc;
	char buf[BUF_SIZE];
	if(args <= 1){
		if((cc = read(STDIN_FILENO, buf, BUF_SIZE)) == -1){
			perror("read");
			exit(1);
		}

		if(write(STDOUT_FILENO, buf, cc) != cc){
			perror("write");
			exit(1);
		}
	} else {
		for(int i = 1; i < args; i++){
			if((fd = open(argv[i], O_RDONLY)) == -1){
				perror("open");
				exit(1);
			}

			int position = 0;
			while((cc = read(fd, buf, BUF_SIZE)) > 0){
				printf("%06x ", position);
				for(int i = 0; i < cc; i++){
					printf("%02x ", buf[i]);
				}

				for(int i = 0; i < (16-cc)*3; i++){
					putchar(' ');
				}

				for(int i = 0; i < cc; i++){
					char c =  buf[i];
					if(!isprint(c)){
						printf(".");
					}
					else{
						printf("%c",c);
					}
				}
				printf("\n");
				position += cc;
			}
			printf("%06x\n", position);

			if(cc == -1){
				perror("read");
				exit(1);
			}

			if (close(fd) == -1){
				perror("close");
				exit(1);
			}
		}
	}
	return 0;
}