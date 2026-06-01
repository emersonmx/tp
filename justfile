set quiet

setup:
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v lefthook &> /dev/null; then
        echo "lefthook not found, installing..."
        go install github.com/evilmartians/lefthook/v2@latest
    fi
    lefthook install

    if ! command -v bacon &> /dev/null; then
        echo "bacon not found, installing..."
        cargo install --locked bacon
    fi

    if ! command -v cargo-nextest &> /dev/null; then
        echo "nextest not found, installing..."
        cargo install --locked cargo-nextest
    fi

    if ! command -v cargo-tarpaulin &> /dev/null; then
        echo "tarpaulin not found, installing..."
        cargo install --locked cargo-tarpaulin
    fi

build *ARGS:
    cargo build {{ ARGS }}

run *ARGS:
    cargo run {{ ARGS }}

watch *ARGS:
    bacon {{ ARGS }}

format *ARGS:
    cargo fmt {{ ARGS }}

lint *ARGS:
    cargo clippy {{ ARGS }}

lint-fix *ARGS:
    cargo clippy --fix --allow-dirty {{ ARGS }}

ci:
    cargo fmt --check
    cargo clippy

test *ARGS:
    cargo nextest {{ ARGS }}

coverage *ARGS:
    cargo tarpaulin {{ ARGS }}

review-snap *ARGS:
    cargo insta review {{ ARGS }}

clean *ARGS:
    cargo clean {{ ARGS }}
