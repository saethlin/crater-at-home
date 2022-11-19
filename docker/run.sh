exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    find /root/build -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +miri update --color=always
        cargo +miri miri test --no-run --color=always --jobs=1 $ARGS
        unbuffer -p /usr/bin/time -v cargo +miri miri nextest run --color=always --no-fail-fast --config-file=/root/.cargo/nextest.toml --jobs=1 $ARGS
        unbuffer -p /usr/bin/time -v timeout $TEST_TIMEOUT cargo +miri miri test --doc --color=always --no-fail-fast --jobs=1 --test-threads=1 $ARGS
        cat Cargo.lock
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
