FC = gfortran

ROOT := ../..
INCLUDE := $(ROOT)/crates/roup-capi/include
LIBDIR := $(ROOT)/target/release
RUST_LIB := $(LIBDIR)/libroup_capi.so
FFLAGS := -std=f2018 -Wall -Wextra -Werror
LDFLAGS := -L$(LIBDIR) -Wl,-rpath,$(abspath $(LIBDIR))
LDLIBS := -lroup_capi -lpthread -ldl -lm
TARGETS := basic_parse tutorial_basic

.PHONY: all clean run-all

all: $(TARGETS)

$(RUST_LIB):
	cd $(ROOT) && cargo build --locked --release -p roup-capi

roup_capi.o roup_capi.mod &: roup_capi.f90 $(RUST_LIB) $(INCLUDE)/roup.h
	$(FC) $(FFLAGS) -c roup_capi.f90

basic_parse: basic_parse.f90 roup_capi.o
	$(FC) $(FFLAGS) basic_parse.f90 roup_capi.o $(LDFLAGS) $(LDLIBS) -o $@

tutorial_basic: tutorial_basic.f90 roup_capi.o
	$(FC) $(FFLAGS) tutorial_basic.f90 roup_capi.o $(LDFLAGS) $(LDLIBS) -o $@

run-all: all
	./basic_parse
	./tutorial_basic

clean:
	rm -f $(TARGETS) *.o *.mod
