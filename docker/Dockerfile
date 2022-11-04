FROM ghcr.io/rust-lang/crates-build-env/linux

ENV PATH=/root/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile=minimal -y && \
    cargo install rustup-toolchain-install-master xargo cargo-nextest && \
    cargo install --git https://github.com/saethlin/miri-tools cargo-download && \
    git clone --depth=1 https://github.com/rust-lang/miri && \
    cd miri && \
    ./miri toolchain && \
    ./miri check && \
    ./miri install && \
    rustup default miri && \
    rustup toolchain remove stable && \
    cd .. && \
    rm -rf miri

RUN apt-get update && \
    apt-get install -y time clang lld expect && \
    rm -rf /var/lib/apt/lists/*

COPY config.toml nextest.toml /root/.cargo/

COPY run.sh get-args.py /root/

ENTRYPOINT ["bash", "/root/run.sh"]
