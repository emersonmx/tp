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
    cargo test {{ ARGS }}

clean *ARGS:
    cargo clean {{ ARGS }}
