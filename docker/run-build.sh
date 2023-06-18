exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    find /root/build /tmp -mindepth 1 -delete # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +$TOOLCHAIN test --no-run --target=x86_64-unknown-linux-gnu $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
