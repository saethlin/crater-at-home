FROM ghcr.io/rust-lang/crates-build-env/linux

ENV PATH=/root/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    cargo install rustup-toolchain-install-master xargo cargo-download && \
    git clone --depth=1 https://github.com/rust-lang/miri && \
    cd miri && \
    ./rustup-toolchain && \
    ./miri check && \
    ./miri install && \
    rustup default miri && \
    rustup toolchain remove stable && \
    cargo +miri miri setup

RUN apt-get update && \
    apt-get install -y time clang lld expect && \
    rm -rf /var/lib/apt/lists/*

COPY config.toml /root/.cargo/config.toml

COPY run.sh /root/run.sh

ENTRYPOINT ["bash", "/root/run.sh"]
