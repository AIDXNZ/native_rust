#!/bin/bash
cargo ndk --manifest-path ../Cargo.toml \
        -t armeabi-v7a \
        -t arm64-v8a \
        -t x86 \
        -t x86_64 \
        -o ../android/app/src/main/jniLibs \
        build --release 
