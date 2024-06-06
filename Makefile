.DEFAULT_GOAL := build-prod


export CARGO_TERM_COLOR=$(shell (test -t 0 && echo "always") || echo "auto")


.PHONY: build-dev
build-dev:
	@rm -f cachebox/*.so
	maturin develop


.PHONY: build-prod
build-prod:
	@rm -f cachebox/*.so
	maturin develop --release


.PHONY: test-py
test-py: build-dev
	python3 -m unittest -v


.PHONY: test-rs
test-rs:
	cargo clippy


.PHONY: test-all
test-all: test-rs test-py


.PHONY: format
format:
	-ruff format --line-length=100 cachebox/
	cargo fmt


.PHONY: clean
clean:
	-ruff clean
	rm -rf `find . -name __pycache__`
	rm -rf cachebox/*.so
