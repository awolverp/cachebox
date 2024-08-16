.DEFAULT_GOAL := all
export CARGO_TERM_COLOR=$(shell (test -t 0 && echo "always") || echo "auto")

.PHONY: build-dev
build-dev:
	maturin develop


.PHONY: build-prod
build-prod:
	maturin develop --release


.PHONY: test-py
test-py:
	maturin develop	
	RUST_BACKTRACE=1 pytest -vv
	rm -rf .pytest_cache
	ruff check .
	ruff clean


.PHONY: test-rs
test-rs:
	cargo clippy


.PHONY: format
format:
	ruff format --line-length=100 .
	ruff clean
	cargo fmt


.PHONY: clean
clean:
	-rm -rf `find . -name __pycache__`
	-rm -rf cachebox/*.so
	-rm -rf target/release


.PHONY: all
all: format test-rs test-py clean
