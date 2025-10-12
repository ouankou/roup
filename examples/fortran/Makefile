# Makefile for ROUP Fortran Examples
# Demonstrates building Fortran programs with ROUP library

.PHONY: all clean help

# Compilers
FC = gfortran
CC = gcc

# Directories
ROUP_ROOT = ../..
ROUP_LIB_DIR = $(ROUP_ROOT)/target/release
ROUP_INCLUDE_DIR = $(ROUP_ROOT)/src

# Flags
FFLAGS = -Wall -g
CFLAGS = -Wall -g
LDFLAGS = -L$(ROUP_LIB_DIR) -lroup -Wl,-rpath,$(ROUP_LIB_DIR)

# Targets
TARGETS = basic_parse tutorial_basic

all: $(TARGETS)

# Build ROUP library first
$(ROUP_LIB_DIR)/libroup.so:
	@echo "Building ROUP library..."
	cd $(ROUP_ROOT) && cargo build --release

# Basic parse example (standalone, no C API calls)
basic_parse: basic_parse.f90
	@echo "Compiling $@..."
	$(FC) $(FFLAGS) -o $@ $<

# Tutorial with C API interoperability
tutorial_basic: tutorial_basic.f90 $(ROUP_LIB_DIR)/libroup.so
	@echo "Compiling $@..."
	$(FC) $(FFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f $(TARGETS) *.o *.mod

help:
	@echo "ROUP Fortran Examples Makefile"
	@echo "=============================="
	@echo ""
	@echo "Targets:"
	@echo "  all              Build all examples (default)"
	@echo "  basic_parse      Simple Fortran example (no API calls)"
	@echo "  tutorial_basic   Full tutorial with C API integration"
	@echo "  clean            Remove built binaries"
	@echo "  help             Show this help message"
	@echo ""
	@echo "Usage:"
	@echo "  make             # Build all examples"
	@echo "  make clean       # Clean build artifacts"
	@echo "  ./basic_parse    # Run basic example"
	@echo "  ./tutorial_basic # Run full tutorial"
