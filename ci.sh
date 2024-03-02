#!/bin/bash

set -e
exec 2>&1
export TERM=xterm-256color

function group {
    echo "::group::$@"
    $@
    echo "::endgroup"
}

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
    group cargo run -- sync --tool="$1" --bucket=miri-bot-dev
    group cargo run -- run --tool="$1" --bucket=miri-bot-dev --crates=10
fi
