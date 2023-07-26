# crater-at-home

> Mom, can I have a crater run?
>
> No, we have crater at home

This project exists because I tried to use [crater](https://github.com/rust-lang/crater) or [rustwide](https://crates.io/crates/rustwide) to run `cargo miri test` on all published crates, and I found it very challenging to add support for Miri, and generally very difficult to hack on those projects.

## Intentional differences with Crater
* Sandboxing restricts writes to anything below the crate root, not the target directory
* Build/test commands are run attached to a pseudoterminal, output is uploaded raw and converted to HTML
* Default resource limits per build are 1 CPU and 8 GB memory
* All state is stored in S3, never on local disk
* No limits on the size of output from builds (some of the logs do get large, but this has not been a problem)

The outcome seems to be significantly greater reliability than crater: We do not see any of the causes of spurious results (apart from crates with flaky test suites) that crater does, and our uploads do not occasionally crash.
I do not understand exactly what causes this to be so much more reliable than crater, but if someone can figure that out it would be great to improve crater.

## Architecture

We sort of have a client-server model.
The server is written in Rust and distributes work one job at a time, and uploads results to S3.
The client is written in bash and runs inside a simple Docker container, which is just an Ubuntu image with a lot of packages installed, a Rust toolchain, and some utilities.
The server `docker run`s a bunch of clients, then writes the names and versions of crates to their stdin, and reads build logs from their stdout.

It is possible to expand the client-server communication to go over the network.
I am not going to implement that, but if it can be done without blowing up the complexity of the project, such a contribution would be welcome.

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
