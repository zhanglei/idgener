.PHONY: build

build:
	cargo build --release

fix:
	cargo fix --all --allow-dirty --allow-staged