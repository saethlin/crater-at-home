FROM ghcr.io/saethlin/crates-build-env:latest

ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y curl build-essential && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- --default-toolchain=nightly --component=miri --component=rust-src --profile=minimal -y && \
    cargo install --git https://github.com/saethlin/miri-tools cargo-download inapty get-args && \
    cargo install cargo-nextest --locked && \
    cargo install --git https://github.com/RalfJung/cargo-careful --rev 7fa90c6 cargo-careful && \
    rm -rf /var/lib/apt/lists/*

COPY nextest.toml /root/.cargo/nextest.toml
COPY run.sh /root/run.sh

RUN mkdir /build && \
    rm -rf /root/.cache && \
    for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do \
        echo "cfg-if@1.0.0" | TOOL=miri TARGET=$target TEST_END_DELIMITER="-" bash /root/run.sh; \
    done && \
    echo "cfg-if@1.0.0" | TOOL=asan TARGET=$(rustc -vV | grep host | cut -d' ' -f2) TEST_END_DELIMITER="-" bash /root/run.sh && \
    tar czvf /cache.tar.gz /root/.cache && \
    rm -rf /root/.cache /build

ENTRYPOINT ["/usr/bin/nice", "-n19", "bash", "/root/run.sh"]
