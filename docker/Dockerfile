FROM ghcr.io/saethlin/crates-build-env:latest

ENV PATH=/root/.cargo/bin:$PATH
ENV TOOLCHAIN=nightly

RUN apt-get update && apt-get install -y curl build-essential && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- --default-toolchain=nightly --component=miri --component=rust-src --profile=minimal -y && \
    cargo install --git https://github.com/saethlin/miri-tools cargo-download inapty get-args && \
    curl -L https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.54/cargo-nextest-0.9.54-x86_64-unknown-linux-gnu.tar.gz | tar xz -C /root/.cargo/bin/ && \
    rm -rf /var/lib/apt/lists/*

COPY nextest.toml /root/.cargo/nextest.toml
COPY run.sh /root/run.sh

ENTRYPOINT ["bash", "/root/run.sh"]
