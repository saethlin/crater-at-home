# Usage Instructions

* install docker (`sudo apt install docker.io`)
* add yourself to docker group (`sudo adduser $USER docker`)
* re-login or `newgrp docker` to make your shell know about docker
* setup docker image (`docker build . -t miri`)
* `cargo run`
* have lots of patience

# Processing the data

All data is collected in `crates.json`. There are currently no convenient tools for
processing this data, but you can get some basic stats via `cargo run --bin stats`.
