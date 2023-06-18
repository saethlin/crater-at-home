exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root/build
    # Delete everything in our writable mount points
    find /root/build /tmp /root/.cargo/registry -mindepth 1 -delete
    if cargo download $crate /root/build
    then
        ARGS=$(python3 /root/get-args.py $crate)
        cargo +$TOOLCHAIN test --no-run --target=x86_64-unknown-linux-gnu $ARGS
    fi
    echo "-${TEST_END_DELIMITER}-"
done < /dev/stdin
