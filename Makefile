.PHONY: build release release-tag install run clean test check fmt clippy coverage coverage-report web-build api-client-build

# The web UI imports @nlindstedt/zig-api-client as a file: dependency,
# which resolves to clients/typescript/dist/. Build the client before
# the web UI so vite can find its entry point on a fresh checkout.
api-client-build:
	cd clients/typescript && npm ci && npm run build

web-build: api-client-build
	cd web && npm ci && npm run build

build: web-build
	cargo build

release: web-build
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
