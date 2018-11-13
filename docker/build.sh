#!/bin/bash -x

REPO=eigenrick
IMAGE=object-spam
TAG=0.3

SOURCEDIR=$(dirname ${BASH_SOURCE[0]})
cd ${SOURCEDIR} && cd ..

docker build -t ${REPO}/${IMAGE}:${TAG} -f docker/Dockerfile .
