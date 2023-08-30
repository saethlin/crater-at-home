#!/bin/bash

set -eu

docker run --rm --privileged tonistiigi/binfmt:latest --install all
docker buildx create --driver docker-container --use
docker buildx build --file ./Dockerfile.base --platform linux/amd64,linux/arm64 --tag ghcr.io/saethlin/crates-build-env:latest --push .
docker buildx build --file ./Dockerfile.ci-base --platform linux/amd64,linux/arm64 --tag ghcr.io/saethlin/crater-at-home-ci:latest --push .
