#!/bin/bash
# for PartID in { 0..4 }
# do
#     # sh build.sh ${PartID}
#     TARGET_NAME=sgx-part${PartID}
#     TARGET_DIR=`pwd`/part${PartID}/target/x86_64-fortanix-unknown-sgx/debug
#     TARGET=$TARGET_DIR/$TARGET_NAME.sgxs
#     # Run enclave with the default runner
#     ftxsgx-runner --signature coresident $TARGET &
# done
# ftxsgx-runner scheduler/target/x86_64-fortanix-unknown-sgx/debug/scheduler.sgxs &
# sleep (5)
# Run SP
(cd ra-sp && cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --example tls-sp --features "verbose") &
# Run client
cd ra-client && cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7710 -s 127.0.0.1:1234 -n 0
cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7711 -s 127.0.0.1:1234 -n 1
cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7712 -s 127.0.0.1:1234 -n 2
cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7713 -s 127.0.0.1:1234 -n 3
cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7714 -s 127.0.0.1:1234 -n 4
cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7719 -s 127.0.0.1:1234 -n 255

# Run client
#(cd ra-client && cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7710 -s 127.0.0.1:1234 -n 0) &
# (cd ra-client && cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --features "verbose" --example tls-client -- -e 127.0.0.1:7711 -s 127.0.0.1:1234  -n 1) &

# Run SP
# (cd ra-sp && cargo run --target x86_64-unknown-linux-gnu -Zfeatures=itarget --example tls-sp --features "verbose") &
