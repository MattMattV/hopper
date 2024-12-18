#!/bin/sh

export HTTP_PORT=4080
export EXTERNAL_BASE=hopper

RUST_BACKTRACE=1 RUST_LOG=debug RUST_LIB_BACKTRACE=1 cargo run

