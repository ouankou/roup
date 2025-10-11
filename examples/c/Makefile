# Makefile for Roup C Examples
#
# This Makefile builds the C example programs that demonstrate the Roup FFI.
# It assumes the Rust library has been built in debug mode.

# Configuration
CC = gcc
CFLAGS = -Wall -Wextra -std=c11
LDFLAGS = -L../../target/debug -lroup -lpthread -ldl -lm
RUST_LIB = ../../target/debug/libroup.so

# Targets
EXAMPLES = basic_parse tutorial_basic

.PHONY: all clean run-all help

all: $(EXAMPLES)

# Build the Rust library first if needed
$(RUST_LIB):
	@echo "Building Rust library..."
	cd ../.. && cargo build --lib
	@echo ""

# Build each example
basic_parse: basic_parse.c $(RUST_LIB)
	@echo "Building $@..."
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

tutorial_basic: tutorial_basic.c $(RUST_LIB)
	@echo "Building $@..."
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

# Run all examples
run-all: all
	@echo "=========================================="
	@echo "Running basic_parse:"
	@echo "=========================================="
	@LD_LIBRARY_PATH=../../target/debug ./basic_parse
	@echo ""
	@echo "=========================================="
	@echo "Running tutorial_basic:"
	@echo "=========================================="
	@LD_LIBRARY_PATH=../../target/debug ./tutorial_basic

# Clean build artifacts
clean:
	@echo "Cleaning..."
	rm -f $(EXAMPLES)

# Show help
help:
	@echo "Roup C Examples - Available targets:"
	@echo ""
	@echo "  make all        - Build all examples"
	@echo "  make run-all    - Build and run all examples"
	@echo "  make clean      - Remove built executables"
	@echo "  make help       - Show this help message"
	@echo ""
	@echo "Individual examples:"
	@echo "  make basic_parse         - Basic parsing and querying"
	@echo "  make tutorial_basic      - Complete tutorial with all API features"
	@echo ""
	@echo "To run an individual example:"
	@echo "  LD_LIBRARY_PATH=../../target/debug ./basic_parse"
	@echo "  LD_LIBRARY_PATH=../../target/debug ./tutorial_basic"
