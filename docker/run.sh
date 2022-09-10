exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    find /root/build -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        cargo +miri update --color=always
        cargo +miri miri test --no-run --color=always --jobs=1
        unbuffer -p /usr/bin/time -v timeout $TEST_TIMEOUT cargo +miri miri test --no-fail-fast -- --test-threads=2
        cat Cargo.lock
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
