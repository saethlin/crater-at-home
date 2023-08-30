#!/bin/bash

set -e
exec 2>&1
export TERM=xterm-256color

function begingroup {
    echo "::group::$@"
}

function endgroup {
    echo "::endgroup"
}

function group {
    echo "::group::$@"
    $@
    echo "::endgroup"
}

begingroup Update toolchain

rustup self update
rustup default stable
rustup update

endgroup

if [[ "$1" == "style" ]]
then
    group cargo fmt --check
    group cargo clippy -- -Dclippy::all
elif [[ "$1" == "fuzz" ]]
then
    group rustup toolchain install nightly
    group cargo install cargo-fuzz
    cd ansi-to-html
    group cargo +nightly fuzz run render -- -runs=100000
else
    group cargo build

    begingroup "Test --tool=$1"
    cargo run -- sync --tool="$1" --bucket=miri-bot-dev
    cargo run -- run --tool="$1" --bucket=miri-bot-dev --crate-list=ci-crates --rerun
    endgroup
fi
