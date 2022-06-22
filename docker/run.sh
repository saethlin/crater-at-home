exec 2>&1

while read crate;
do
    cd /root # ensure that we don't remove the current directory
    rm -rf /root/build # clean out anything from an old build (probably)
    if cargo download --extract --output=/root/build $crate
    then
        cd /root/build
        cargo +miri update --color=always
        cargo +miri miri test --no-run --color=always
        /usr/bin/time -v timeout $TEST_TIMEOUT cargo +miri miri test --color=always --no-fail-fast -- --test-threads=2
        cat Cargo.lock
        echo $TEST_END_DELIMITER
    fi
done < /dev/stdin
