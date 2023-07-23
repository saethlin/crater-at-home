FROM ubuntu:latest AS builder

# Install the packages contained in `packages.txt`
COPY packages.txt /packages.txt
RUN apt-get update && \
    cat /packages.txt | DEBIAN_FRONTEND=noninteractive xargs apt-get install -y && \
    rm -rf /var/lib/apt/lists/* && \
    rm /packages.txt

FROM builder

ENV PATH=/root/.cargo/bin:$PATH
ENV TOOLCHAIN=nightly

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- --default-toolchain=nightly --component=miri --component=rust-src --profile=minimal -y && \
    cargo install --git https://github.com/saethlin/miri-tools cargo-download inapty get-args && \
    cargo install cargo-nextest

COPY run.sh /root/run.sh

ENV RUSTFLAGS="-Zsanitizer=address --cap-lints=allow -Zrandomize-layout -Copt-level=0 -Cdebuginfo=1 -Zvalidate-mir"
ENV RUSTDOCFLAGS=$RUSTFLAGS
ENV ASAN_OPTIONS=detect_stack_use_after_return=true:allocator_may_return_null=1:detect_invalid_pointer_pairs=2
ENV CARGO_INCREMENTAL=0
ENV RUST_BACKTRACE=1

ENTRYPOINT ["bash", "/root/run.sh"]
