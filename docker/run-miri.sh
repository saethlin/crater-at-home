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
        MIRIFLAGS="$MIRIFLAGS --color=always" unbuffer -p cargo +miri miri nextest run --color=always --no-fail-fast --config-file=/root/.cargo/nextest.toml --jobs=1 $ARGS
        unbuffer -p cargo +miri miri test --doc --no-fail-fast --jobs=1 $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
