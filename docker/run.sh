exec 2>&1

export TERM=xterm-256color

while read crate;
do
    cd /root # ensure that we don't remove the current directory
    rm -rf /root/build # clean out anything from an old build (probably)
    if cargo download $crate /root/build
    then
        cd /root/build
        cargo +miri update --color=always
        cargo +miri miri test --no-run --color=always --jobs=1
        unbuffer -p /usr/bin/time -v timeout $TEST_TIMEOUT cargo +miri miri test --no-fail-fast -- --test-threads=2
        cat Cargo.lock
    fi
    echo $TEST_END_DELIMITER
done < /dev/stdin
