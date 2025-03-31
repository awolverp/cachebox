help:
	@echo "Commands:"
	@echo -e "\tbuild-dev       build source"
	@echo -e "\tbuild-prod      build source (release mode)"
	@echo -e "\ttest-rs         clippy and test rust code"
	@echo -e "\ttest-py         build and test python code"
	@echo -e "\tformat          format rust and python code"
	@echo -e "\tclean           clean all the unneeded files"

.PHONY: build-dev
build-dev:
	maturin develop

.PHONY: build-prod
build-prod:
	maturin develop --release

.PHONY: test-rs
test-rs:
	cargo clippy
	cargo test -- --nocapture

.PHONY: test-py
test-py: build-dev
	coverage run -m pytest -s -vv
	-rm -rf .pytest_cache
	-ruff check .
	ruff clean
	coverage html

.PHONY: format
format:
	ruff format --line-length=100 .
	ruff clean
	cargo fmt

.PHONY: clean
clean:
	-rm -rf `find . -name __pycache__`
	-rm -rf python/cachebox/*.so
	-rm -rf target/release
	-rm -rf .pytest_cache
	-rm -rf .coverage
	-rm -rf htmlcov
	-ruff clean
