FROM ubuntu:latest

# Install the packages contained in `packages.txt`
COPY packages.txt /packages.txt
RUN apt-get update && \
    cat /packages.txt | DEBIAN_FRONTEND=noninteractive xargs apt-get install -y && \
    rm -rf /var/lib/apt/lists/* && \
    rm /packages.txt
