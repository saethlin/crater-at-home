exec 2>&1

export TERM=xterm-256color

# Use 1 GB for a default stack size.
# We really want to only run out of stack in true infinite recursion.
ulimit -s 1048576
export RUST_MIN_STACK=1073741824

while read crate;
do
    cd /root/build
    # Delete everything in our writable mount points
    find /root/build /tmp /root/.cargo/registry -mindepth 1 -delete
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +nightly update &> /dev/null
        cargo +nightly test --no-run --target=x86_64-unknown-linux-gnu $ARGS &> /dev/null
        timeout --kill-after=10 600 unbuffer -p cargo +nightly test --color=always --no-fail-fast --target=x86_64-unknown-linux-gnu $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
