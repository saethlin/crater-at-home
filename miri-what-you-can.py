"""
Run the test suite for a Rust crate under Miri
"""
import sys
import os
import subprocess
import shutil
import json

os.chdir("/tmp")

output = subprocess.run(
    ["cargo", "download", "-x", sys.argv[1]],
    capture_output=True,
)
output = output.stderr.decode("utf-8")
crate_dir = output.splitlines()[-1].split()[-1]
os.chdir(crate_dir)

output = subprocess.run(
    ["cargo", "+nightly", "test", "--", "--list"], capture_output=True
)
output.check_returncode()
output = output.stdout.decode("utf-8")

tests = set()
for line in output.splitlines():
    line = line.strip()
    if line.endswith(": test") and len(line.split()) == 2:
        tests.add(line[:-6])

tests = sorted(list(tests))

os.environ[
    "MIRIFLAGS"
] = "-Zmiri-ignore-leaks -Zmiri-disable-isolation -Zmiri-symbolic-alignment-check -Zmiri-tag-raw-pointers -Zmiri-check-number-validity"
os.environ["RUSTFLAGS"] = "-Zrandomize-layout"
while True:
    output = subprocess.run(
        [
            "cargo",
            "+nightly",
            "miri",
            "test",
            "--",
            "-Zunstable-options",
            "--format=json",
            "--exact",
        ]
        + tests,
        capture_output=True,
    )
    if output.returncode == 0:
        break
    errors = output.stderr.decode("utf-8")
    output = output.stdout.decode("utf-8")
    if any(
        l.startswith("error: unsupported operation:")
        for l in errors.strip().splitlines()
    ):
        for line in output.strip().splitlines():
            line = json.loads(line)
            if (line["type"] == "test") and (line["event"] == "started"):
                if line["name"] in tests:
                    tests.remove(line["name"])
        if not tests:
            break
    else:
        print(errors)
        last_line = output.strip().splitlines()[-1]
        failing_test = json.loads(last_line)["name"]
        print("Failing test found: {}, reproduce with:".format(failing_test))
        print(
            "MIRIFLAGS={} ".format(os.environ["MIRIFLAGS"])
            + "cargo +nightly miri test -- --exact {}".format(
                failing_test
            )
        )
        break
