#!/bin/bash

set -eu

# Start up docker-buildx
docker run --rm --privileged tonistiigi/binfmt:latest --install all
docker buildx create --driver docker-container --use

# Full base image is just all the packages in packages.txt
docker buildx build --no-cache --file ./Dockerfile.base --platform linux/amd64,linux/arm64 --tag ghcr.io/saethlin/crates-build-env:latest --push .

# CI base image is the full image, but with no packages.
echo "FROM ubuntu:latest" > Dockerfile.ci-base
tail -n+2 Dockerfile >> Dockerfile.ci-base
docker buildx build --no-cache --file ./Dockerfile.ci-base --platform linux/amd64 --tag ghcr.io/saethlin/crater-at-home-ci:latest --push .
