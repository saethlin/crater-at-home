exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    find /root/build -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 get-args.py)
        cargo +miri update --color=always
        cargo +miri miri test --no-run --color=always --jobs=1 $ARGS
        unbuffer -p /usr/bin/time -v cargo +miri miri nextest run --no-fail-fast --config-file=/root/.cargo/nextest.toml $ARGS
        cat Cargo.lock
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
