BUILD_TYPE ?= release
CXX ?= c++

ROOT := ../..
INCLUDE := $(ROOT)/crates/roup-capi/include
LIBDIR := $(ROOT)/target/$(BUILD_TYPE)
RUST_LIB := $(LIBDIR)/libroup_capi.so
CXXFLAGS := -std=c++17 -Wall -Wextra -Werror -I$(INCLUDE)
LDFLAGS := -L$(LIBDIR)
LDLIBS := -lroup_capi -lpthread -ldl -lm
EXAMPLES := tutorial_basic

.PHONY: all clean run-all

all: $(EXAMPLES)

$(RUST_LIB):
	cd $(ROOT) && cargo build --locked -p roup-capi $(if $(filter release,$(BUILD_TYPE)),--release,)

%: %.cpp $(RUST_LIB) $(INCLUDE)/roup.h
	$(CXX) $(CXXFLAGS) $< $(LDFLAGS) $(LDLIBS) -o $@

run-all: all
	LD_LIBRARY_PATH=$(LIBDIR) ./tutorial_basic

clean:
	rm -f $(EXAMPLES)
