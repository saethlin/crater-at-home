# crater-at-home

> Mom, can I have a crater run?
> No, we have crater at home

This is a tool for building or running tests for all crates published to crates.io.
Unlike crater, it is not at all designed to look for regressions.

This project exists because I tried to use `crater` or `rustwide` to run `cargo miri test` on all published crates, and I found those projects exceptionally difficult to hack on.
To that end, the approach of this project is rather different.
We do not try to do anything clever with caching, or with fine-grained sandboxing, and do not have runtime control of toolchains.

The outcome seems to be significantly greater reliability than crater; we do not see any of the causes of spurious results (apart from crates with flaky test suites) that crater does, and our uploads do not occasionally crash.
I do not understand exactly what causes this to be so much more reliable than crater; if you know better I'm game to send patches to crater.

## Architecture

We sort of have a client-server model.
The server is written in Rust and distributes work one job at a time, and uploads results to S3.
The client is written in bash and runs inside a simple Docker container, which is just an Ubuntu image with a lot of packages installed, a Rust toolchain, and some utilities.
The server `docker run`s a bunch of clients, then writes the names and versions of crates to their stdin, and reads build logs from their stdout.

It would probably be possible to expand that communication to go over the network.
I am not interested in making that happen, but if it can be done without blowing up the complexity of the project, such a contribution would be welcome.

## Sandboxing

We provide sandboxing by mounting the client containers as read-only, with exceptions for `/tmp`, the Cargo registry, and the entire directory crates are built in.
Unlike crater, if a crate's build system tries to write outside of its target directory, that is fine.
The build directory is cleaned out between crates without taking down the container.

## Resource limits

Every container is pinned to a single CPU with a cpuset.
There are some crates that for whatever reason don't respect this and try to launch 2 or 64 jobs.
Lol.

Memory is very different.
About 10% of crates need 2 GB peak, and in Miri that number is significantly higher.
We impose a memory limit of 8 GB per container to prevent runaway resource usage, but in general it is strongly advised that you run this program heavily oversubscribed.
On *average*, a container needs less than 1 GB, but averages are only relevant if outliers cannot be significant.
It is generally advised to run this on a large system; I normally run it on a system with 64 CPUs (my desktop or a c6.16xlarge instance).

## Usage Suggestions

This crate uses `clap`; run `cargo run -- --help` for CLI options.

* Install docker (`sudo apt install docker.io`, `sudo pacman -S docker`, etc.)
* Add yourself to docker group (`sudo adduser $USER docker`)
* Re-login or `newgrp docker` to make your shell know about docker
* `cargo run -- run --tool=miri --bucket=my-bucket-here`
* Have lots of patience

Contributions of or suggestions for more sophisticated data processing are welcome.
