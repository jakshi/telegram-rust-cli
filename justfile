set shell := ["bash", "-cu"]

import? 'justfile.local'

default:
    @just --list

fmt:
    cargo fmt --check

test:
    cargo test

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

audit:
    cargo audit

fast-check: fmt clippy

check: fmt test clippy audit

fix:
    cargo fmt
    cargo clippy --fix --all-targets --all-features --allow-dirty -- -D warnings
