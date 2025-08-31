#!/usr/bin/env bash
export SOURCE_DATE_EPOCH="$(git log -1 --pretty=%ct 2>/dev/null || date +%s)"
export RUSTFLAGS="--remap-path-prefix=$(pwd)=/source"
export CARGO_TERM_COLOR=never

