#!/bin/bash

set -e
exec 2>&1
export TERM=xterm-256color

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

rustup self update
rustup default stable
rustup update

$1
