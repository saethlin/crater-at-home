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
RUN rustup default miri
RUN rustup toolchain remove stable
RUN rm -rf /miri
WORKDIR /root
RUN echo "cargo download -x --output=/root/build 2>&1 \$1 && cd /root/build && cargo miri test --jobs=1 --no-fail-fast -- --test-threads=1 2>&1" >> run.sh
ENTRYPOINT ["bash", "run.sh"]
