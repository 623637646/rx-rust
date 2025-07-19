#!/bin/bash

cargo fmt
cargo hack --each-feature --no-dev-deps --exclude-all-features check
cargo hack --each-feature --exclude-all-features --exclude-no-default-features --exclude-features default,multi-threaded,single-threaded check --all-targets