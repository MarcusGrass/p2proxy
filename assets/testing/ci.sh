#/bin/sh
set -e
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo r -p p2proxy-test-runner
cargo test --all-features
cargo deny check
gitleaks --verbose detect