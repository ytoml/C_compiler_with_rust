#!/bin/zsh

MOUNT_PATH=$(pwd)
ASM_SRC=$1
LINK_SRC="tests/utils/helper.c"

# c_exec はイメージ名
docker run --rm \
    -v $MOUNT_PATH:$MOUNT_PATH \
    --platform linux/amd64 \
	c_exec \
    /bin/bash -c \
    "gcc -o /tmp/tmp ${MOUNT_PATH}/${ASM_SRC} ${MOUNT_PATH}/${LINK_SRC};
    /tmp/tmp;
    echo \$?;
    rm /tmp/tmp;
    "
