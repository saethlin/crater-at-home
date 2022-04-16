FROM ghcr.io/rust-lang/crates-build-env/linux

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN git clone --branch=track-alloc-history --depth=1 https://github.com/saethlin/miri
WORKDIR miri
ENV PATH=/root/.cargo/bin:$PATH
RUN cargo install rustup-toolchain-install-master
RUN cargo install xargo
RUN cargo install cargo-download
RUN ./rustup-toolchain
RUN ./miri check
RUN ./miri install
RUN rm -rf /miri
RUN rustup default miri
RUN rustup toolchain remove stable
RUN apt-get update && apt-get install -y time
WORKDIR /root
RUN echo "exec 2>&1" >> run.sh && \
    echo "set -v" >> run.sh && \
    echo "cargo download --extract --output=/root/build \$1 || exit 1" >> run.sh && \
    echo "cd build" >> run.sh && \
    echo "cargo +miri update" >> run.sh && \
    echo "/usr/bin/time -v timeout 900 cargo +miri miri test --jobs=1 --no-fail-fast -- --test-threads=1" >> run.sh && \
    echo "cat Cargo.lock" >> run.sh
ENTRYPOINT ["bash", "run.sh"]
