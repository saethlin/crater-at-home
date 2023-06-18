exec 2>&1

export TERM=xterm-256color

cargo +nightly miri setup &> /dev/null

while read crate;
do
    cd /root/build
    find /root/build /tmp -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +nightly update &> /dev/null
        cargo +nightly miri test --no-run $ARGS &> /dev/null
        timeout --kill-after=10 3600 unbuffer -p cargo +nightly miri nextest run --color=always --no-fail-fast --config-file=/root/.cargo/nextest.toml $ARGS
        timeout --kill-after=10 600 unbuffer -p cargo +nightly miri test --doc --no-fail-fast $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
