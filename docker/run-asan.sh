exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    find /root/build -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +nightly update --color=always
        cargo +nightly careful test --no-run --color=always $ARGS
        unbuffer -p /usr/bin/time -v cargo +nightly careful test --no-fail-fast -- --test-threads=2 $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
