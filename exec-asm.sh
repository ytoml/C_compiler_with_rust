#! /bin/zsh

MOUNT_PATH=$(pwd)
IMAGE_NAME=$1
ASM_SRC=$2

docker run --rm  -it\
    -v $MOUNT_PATH:$MOUNT_PATH \
    --platform linux/amd64 \
    $IMAGE_NAME \
    sh -c \
    "
    gcc -o ./tmp/tmp $MOUNT_PATH/$ASM_SRC;
    ./tmp/tmp;
    rm tmp/tmp;
    echo $?;
    "
