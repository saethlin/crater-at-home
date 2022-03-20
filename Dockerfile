FROM ghcr.io/rust-lang/crates-build-env/linux

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN git clone --branch=track-alloc-history --depth=1 https://github.com/saethlin/miri
WORKDIR miri
ENV PATH=/root/.cargo/bin:$PATH
RUN cargo install rustup-toolchain-install-master
RUN ./rustup-toolchain
RUN ./miri check
RUN ./miri install
RUN rustup default miri
RUN rustup toolchain remove stable
WORKDIR /root
