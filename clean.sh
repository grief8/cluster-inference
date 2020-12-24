for i in 0 1 2 3 4;
do
    (cd part${i} && cargo clean)
done;
# Build and sign enclave
(cd ra-enclave && cargo clean)

# Build client
(cd ra-client && cargo clean)

# Build SP
(cd ra-sp && cargo clean)
