.DEFAULT_GOAL := all


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
test-py:
	python3 -m unittest



.PHONY: test-rs
test-rs:
	cargo clippy
	cargo check


.PHONY: test-all
test-all: test-rs test-py


.PHONY: clean
clean:
	rm -rf `find . -name __pycache__`
	rm -f `find . -type f -name '*.py[co]' `
	rm -f `find . -type f -name '*~' `
	rm -f `find . -type f -name '.*~' `
	rm -rf .cache
	rm -rf flame
	rm -rf *.egg-info
	rm -rf build
	rm -rf perf.data*
	rm -rf cachebox/*.so

.PHONY: bench
bench:
	python3 benchmarks
