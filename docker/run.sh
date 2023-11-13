set -u
exec 2>&1

export TERM=xterm-256color

# Extract the cache if it exists
# Ideally it's just an error to not have a cache, but this script is executed to build the cache.
if [ -e /cache.tar.gz ]
then
    tar xf /cache.tar.gz
fi

TOOLCHAIN=nightly
HOST=$(rustc +$TOOLCHAIN -vV | grep host | rev | cut -d' ' -f1 | rev)

export CARGO_INCREMENTAL=0
export RUST_BACKTRACE=1
export RUSTFLAGS="--cap-lints=warn -Copt-level=0 -Zvalidate-mir"
if [[ $HOST == "x86_64-unknown-linux-gnu" ]]; then
    export RUSTFLAGS="$RUSTFLAGS -Ctarget-cpu=x86-64-v2"
fi

if [[ $TOOL == "build" ]]; then
    export RUSTFLAGS="-Zvalidate-mir -Zmir-opt-level=4 -Zinline-mir -Cdebuginfo=2 -Cdebug-assertions=yes -Copt-level=3 -Zcross-crate-inline-threshold=always -Zthreads=64 -Zinline-mir-hint-threshold=10000 -Zinline-mir-threshold=10000"
elif [[ $TOOL == "asan" ]]; then
    # Use 1 GB for a default stack size.
    # We really want to only run out of stack in true infinite recursion.
    ulimit -s 1048576
    export RUST_MIN_STACK=1073741824
    export RUSTFLAGS="$RUSTFLAGS -Cdebuginfo=1 -Zstrict-init-checks=no"
    export ASAN_OPTIONS="color=always:detect_leaks=0:detect_stack_use_after_return=true:allocator_may_return_null=1:detect_invalid_pointer_pairs=2"
elif [[ $TOOL == "miri" ]]; then
    export RUSTFLAGS="$RUSTFLAGS -Zrandomize-layout -Cdebuginfo=0"
    export MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-num-cpus=64"
fi
export RUSTDOCFLAGS=$RUSTFLAGS

function timed {
    timeout --kill-after=10s 1h inapty cargo +$TOOLCHAIN "$@" --target=$HOST
}

function run_build {
    timed test --no-run $ARGS
}

function run_check {
    timed check $ARGS
}

function run_asan {
    timed careful test -Zcareful-sanitizer=address --no-run $ARGS &> /dev/null
    timed careful test -Zcareful-sanitizer=address --color=always --no-fail-fast $ARGS
}

function run_miri {
    timed miri test --no-run $ARGS &> /dev/null
    # rustdoc is already passed --color=always, so adding it to the global MIRIFLAGS is just an error
    MIRIFLAGS="$MIRIFLAGS --color=always" timed miri nextest run --color=always --no-fail-fast --config-file=/root/.cargo/nextest.toml $ARGS
    # nextest runs one interpreter per test, so unsupported errors only terminate the test not the whole suite.
    # But we need to panic on unsupported for doctests, because nextest doesn't support doctests.
    MIRIFLAGS="$MIRIFLAGS -Zmiri-panic-on-unsupported" timed miri test --doc --no-fail-fast $ARGS
}

if [[ $TOOL == "miri" ]]; then
    timed miri setup &> /dev/null
fi

while read crate;
do
    cd /build
    # Delete everything in our writable mount points
    find /build /tmp /root/.cargo/registry -mindepth 1 -delete
    if cargo download $crate /build; then
        ARGS=$(get-args $crate)
        cargo update &> /dev/null
        if [[ $TOOL == "build" ]]; then
            run_build
        elif [[ $TOOL == "check" ]]; then
            run_check
        elif [[ $TOOL == "asan" ]]; then
            run_asan
        elif [[ $TOOL == "miri" ]]; then
            run_miri
        else
            exit 1
        fi
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
