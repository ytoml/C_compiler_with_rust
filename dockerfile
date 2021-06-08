FROM --platform=linux/amd64 ubuntu:20.04

RUN set -x &&\
    apt update &&\
    apt -y install sudo

RUN useradd -m docker &&\
    echo "docker:docker" | chpasswd &&\
    adduser docker sudo

USER docker
CMD /bin/sh