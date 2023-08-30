#!/bin/bash

set -e
exec 2>&1
export TERM=xterm-256color

function begingroup {
    echo "::group::$@"
    set -x
}

function endgroup {
    set +x
    echo "::endgroup"
}

function style() {
    cargo fmt --check
    cargo clippy -- -Dclippy::all
}

function miri() {
    cargo run -- sync --tool=miri --bucket=miri-bot-dev
    cargo run -- run --tool=miri --bucket=miri-bot-dev --crate-list=ci-crates --rerun
}

function asan() {
    cargo run -- sync --tool=asan --bucket=miri-bot-dev
    cargo run -- run --tool=asan --bucket=miri-bot-dev --crate-list=ci-crates --rerun
}

function build() {
    cargo run -- sync --tool=build --bucket=miri-bot-dev
    cargo run -- run --tool=build --bucket=miri-bot-dev --crate-list=ci-crates --rerun
}

begingroup Update toolchain

rustup self update
rustup default stable
rustup update

endgroup

if $1 -ne "style"
then
    begingroup cargo build
    cargo build
    endgroup
fi

$1
