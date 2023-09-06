#!/bin/bash
#cargo build --release
cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target x86_64-unknown-linux-gnu --release
ls -lha ./target/x86_64-unknown-linux-gnu/release/yjump

./target/x86_64-unknown-linux-gnu/release/yjump