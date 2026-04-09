.PHONY: build release release-tag install run clean test check fmt clippy coverage coverage-report

build:
	cargo build

release:
	cargo build --release

release-tag:
	scripts/release.sh $(BUMP)

install:
	cargo install --path zig-cli

run:
	cargo run --bin zig

clean:
	cargo clean

test:
	cargo test --workspace

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

coverage:
	cargo llvm-cov --workspace --summary-only

coverage-report:
	cargo llvm-cov --workspace --html --output-dir .coverage/html
