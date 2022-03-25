FROM ghcr.io/rust-lang/crates-build-env/linux

ENV PATH=/root/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    git clone --branch=track-alloc-history --depth=1 https://github.com/saethlin/miri && \
    cd miri && \
    cargo install rustup-toolchain-install-master && \
    cargo install xargo && \
    cargo install cargo-download && \
    ./rustup-toolchain && \
    ./miri check && \
    ./miri install && \
    cd / && \
    rm -rf /miri && \
    rustup default miri && \
    rustup toolchain remove stable && \
    apt-get update && apt-get install -y time && \
    echo "exec 2>&1" >> run.sh && \
    echo "set -v" >> run.sh && \
    echo "cargo download --extract --output=/root/build \$1 || exit 1" >> run.sh && \
    echo "cd build" >> run.sh && \
    echo "/usr/bin/time -v timeout 900 cargo miri test --jobs=1 --no-fail-fast -- --test-threads=1" >> run.sh && \
    echo "cat Cargo.lock" >> run.sh

ENTRYPOINT ["bash", "run.sh"]
