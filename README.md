This repo contains tools for running Miri and generating the HTML for
https://miri.saethlin.dev. You can browse the build logs for just the crates
where Miri reports UB at https://miri.saethlin.dev/ub.

# Usage Instructions

This crate uses `clap`; run `cargo run -- --help` for CLI options.

* install docker (`sudo apt install docker.io`, `sudo pacman -S docker`, etc.)
* add yourself to docker group (`sudo adduser $USER docker`)
* re-login or `newgrp docker` to make your shell know about docker
* setup docker image (`docker build -t miri - < Dockerfile`)
* `cargo run -- --crates=n`
* have lots of patience

# Processing the data

All data is output to the directory `logs/` in the root of the repo.
You can get some basic stats via `cargo run --bin stats`, and render an HTML
page for every crate plus two summary pages with `cargo run --bin render`.

Contributions of or suggestions for more sophisticated data processing are welcome.
