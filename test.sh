#!/bin/bash

#set -x
set -e

cd "$(dirname "$0")"
source test_utils.sh

setup_test_runner

echo "Testing crates..."
test_crates_features \
    "sbor \
    sbor-derive-common \
    sbor-derive \
    sbor-tests \
    scrypto \
    scrypto-derive \
    scrypto-tests \
    radix-engine-derive \
    radix-engine-common \
    radix-engine-interface \
    radix-engine \
    radix-engine-tests \
    transaction"

echo "Testing scrypto packages..."
test_packages \
    "assets/blueprints/faucet \
    examples/hello-world \
    examples/no-std"

echo "Testing CLIs..."
(cd simulator; RUST_MIN_STACK=4194304 cargo test)
test_cli \
    "./tests/resim.sh \
    ./tests/scrypto.sh \
    ./tests/manifest.sh"

echo "Running benchmark..."
test_benchmark  \
    "sbor-tests \
    radix-engine-tests"

echo "Congrats! All tests passed."
