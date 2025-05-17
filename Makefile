.PHONY: build test run clean

# Build the compiler
build:
	cargo build

# Run tests
test:
	cargo test

# Run a Clean Language program
run:
	cargo run --bin clean-language-compiler -- -i $(INPUT) -o $(OUTPUT)

# Clean build artifacts
clean:
	cargo clean

# Example usage:
# make run INPUT=examples/hello.cln OUTPUT=examples/hello.wasm 