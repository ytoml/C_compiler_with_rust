FROM --platform=linux/amd64 ubuntu:latest

# Rui UeyamaさんのDockerfile(https://www.sigbus.info/compilerbook/Dockerfile)を参考にしています。
RUN apt update
RUN DEBIAN_FRONTEND=noninteractive apt install -y gcc make git binutils libc6-dev gdb sudo

CMD /bin/sh