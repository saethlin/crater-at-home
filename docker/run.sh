set -u
exec 2>&1

export TERM=xterm-256color

export CARGO_INCREMENTAL=0
export RUST_BACKTRACE=1
export RUSTFLAGS="--cap-lints=warn -Copt-level=0 -Zvalidate-mir"
if [[ $TOOL == "build" ]]; then
    export RUSTFLAGS="$RUSTFLAGS -Zmir-opt-level=4 -Zinline-mir -Cdebuginfo=2 -Cdebug-assertions=yes"
elif [[ $TOOL == "asan" ]]; then
    # Use 1 GB for a default stack size.
    # We really want to only run out of stack in true infinite recursion.
    ulimit -s 1048576
    export RUST_MIN_STACK=1073741824
    export RUSTFLAGS="$RUSTFLAGS -Cdebuginfo=1 -Zstrict-init-checks=no"
    export ASAN_OPTIONS="color=always:detect_leaks=0:detect_stack_use_after_return=true:allocator_may_return_null=1:detect_invalid_pointer_pairs=2"
elif [[ $TOOL == "miri" ]]; then
    export RUSTFLAGS="$RUSTFLAGS -Zrandomize-layout -Cdebuginfo=0"
    export MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-panic-on-unsupported"
fi
export RUSTDOCFLAGS=$RUSTFLAGS

TOOLCHAIN=nightly

HOST=$(rustc +$TOOLCHAIN -vV | grep host | rev | cut -d' ' -f1 | rev)

function run_build {
    inapty cargo +$TOOLCHAIN test --no-run --target=$HOST $ARGS
}

function run_asan {
    cargo +$TOOLCHAIN careful test -Zcareful-sanitizer=address --no-run --target=$HOST $ARGS &> /dev/null
    timeout --kill-after=10 600 inapty cargo +$TOOLCHAIN careful test -Zcareful-sanitizer=address --color=always --no-fail-fast --target=$HOST $ARGS
}

function run_miri {
    cargo +$TOOLCHAIN miri setup
    cargo +$TOOLCHAIN miri test --no-run $ARGS &> /dev/null
    # rustdoc is already passed --color=always, so adding it to the global MIRIFLAGS is just an error
    MIRIFLAGS="$MIRIFLAGS --color=always" timeout --kill-after=10 3600 inapty cargo +$TOOLCHAIN miri nextest run --color=always --no-fail-fast --config-file=/root/.cargo/nextest.toml $ARGS
    timeout --kill-after=10 600 inapty cargo +$TOOLCHAIN miri test --doc --no-fail-fast $ARGS
}

if [[ $TOOL == "miri" ]]; then
    cargo +$TOOLCHAIN miri setup &> /dev/null
fi

while read crate;
do
    cd /root/build
    # Delete everything in our writable mount points
    find /root/build /tmp /root/.cargo/registry -mindepth 1 -delete
    if cargo download $crate /root/build; then 
        ARGS=$(get-args $crate)
        cargo +$TOOLCHAIN update &> /dev/null
        if [[ $TOOL == "build" ]]; then
            run_build
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
