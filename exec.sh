#!/bin/zsh

MOUNT_PATH=$(pwd)
IMAGE_NAME=$1
OBJECT_NAME=$2

docker run --rm \
    -v $MOUNT_PATH:$MOUNT_PATH \
    --platform linux/amd64 \
    $IMAGE_NAME \
    sh -c \
    "cd $MOUNT_PATH;
    ./$OBJECT_NAME;
    "
