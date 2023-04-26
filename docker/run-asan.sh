exec 2>&1

export TERM=xterm-256color

cargo +nightly careful setup &> /dev/null

while read crate;
do
    cd /root/build
    find /root/build -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +nightly update &> /dev/null
        cargo +nightly careful test --no-run --jobs=1 $ARGS &> /dev/null
        timeout --kill-after=10 600 unbuffer -p cargo +nightly careful test --jobs=1 --no-fail-fast $ARGS -- --test-threads=1
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
