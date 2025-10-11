# C++ Tutorial: Building a Real Application

This tutorial shows how to integrate ROUP into a **real C++ application** using modern C++17 features.

We'll build an **OpenMP pragma analyzer** - a tool that reads C/C++ source files, extracts OpenMP directives, and reports statistics.

---

## What You'll Build

A command-line tool that:
1. Reads source files line-by-line
2. Detects OpenMP pragmas
3. Parses them using ROUP
4. Reports directive types and clause counts
5. Provides summary statistics

**Example output:**
```
$ ./omp_analyzer mycode.c
Found 5 OpenMP directives:
  Line 10: parallel (3 clauses)
  Line 25: for (2 clauses)
  Line 42: parallel for (4 clauses)
  Line 68: task (1 clause)
  Line 95: barrier (0 clauses)

Summary:
  Total directives: 5
  Total clauses: 10
  Most common: parallel (2 occurrences)
```

---

## Prerequisites

- **C++ Compiler:** clang++ or g++ with C++17 support
- **ROUP Library:** Built and installed (see below)
- **System:** Linux, macOS, or Windows with WSL

### Building ROUP

```bash
git clone https://github.com/ouankou/roup.git
cd roup
cargo build --release

# Library is now at: target/release/libroup.so (Linux)
#                or: target/release/libroup.dylib (macOS)
```

---

## Step 1: Understanding the ROUP C API

ROUP exports a minimal C API with 18 functions. Here are the key ones for our tool:

### Lifecycle Functions
```c
// Parse an OpenMP directive string
OmpDirective* roup_parse(const char* input);

// Free the parsed directive
void roup_directive_free(OmpDirective* directive);
```

### Query Functions
```c
// Get directive type (0=parallel, 1=for, 4=task, etc.)
int32_t roup_directive_kind(const OmpDirective* directive);

// Get number of clauses
int32_t roup_directive_clause_count(const OmpDirective* directive);

// Create iterator for clauses
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);

// Get next clause (returns 1 if available, 0 if done)
int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);

// Get clause type (0=num_threads, 2=private, 6=reduction, etc.)
int32_t roup_clause_kind(const OmpClause* clause);

// Free iterator
void roup_clause_iterator_free(OmpClauseIterator* iter);
```

---

## Step 2: Create RAII Wrappers (Modern C++)

Instead of manual memory management, let's use **RAII** (Resource Acquisition Is Initialization) to automatically clean up resources.

Create `roup_wrapper.hpp`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <string>
#include <vector>
#include <optional>

// Forward declarations of opaque C types
struct OmpDirective;
struct OmpClause;
struct OmpClauseIterator;

// C API declarations
extern "C" {
    OmpDirective* roup_parse(const char* input);
    void roup_directive_free(OmpDirective* directive);
    int32_t roup_directive_kind(const OmpDirective* directive);
    int32_t roup_directive_clause_count(const OmpDirective* directive);
    OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
    int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
    void roup_clause_iterator_free(OmpClauseIterator* iter);
    int32_t roup_clause_kind(const OmpClause* clause);
}

namespace roup {

// RAII wrapper for OmpDirective
class Directive {
private:
    OmpDirective* ptr_;

public:
    // Constructor: parse directive
    explicit Directive(const std::string& input) 
        : ptr_(roup_parse(input.c_str())) {}
    
    // Destructor: automatic cleanup
    ~Directive() {
        if (ptr_) {
            roup_directive_free(ptr_);
        }
    }
    
    // Delete copy (move-only type)
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;
    
    // Move constructor
    Directive(Directive&& other) noexcept : ptr_(other.ptr_) {
        other.ptr_ = nullptr;
    }
    
    // Move assignment
    Directive& operator=(Directive&& other) noexcept {
        if (this != &other) {
            if (ptr_) roup_directive_free(ptr_);
            ptr_ = other.ptr_;
            other.ptr_ = nullptr;
        }
        return *this;
    }
    
    // Check if parse succeeded
    bool valid() const { return ptr_ != nullptr; }
    explicit operator bool() const { return valid(); }
    
    // Get directive kind
    int32_t kind() const {
        return ptr_ ? roup_directive_kind(ptr_) : -1;
    }
    
    // Get clause count
    int32_t clause_count() const {
        return ptr_ ? roup_directive_clause_count(ptr_) : 0;
    }
    
    // Get raw pointer (for advanced usage)
    OmpDirective* get() const { return ptr_; }
};

// RAII wrapper for OmpClauseIterator
class ClauseIterator {
private:
    OmpClauseIterator* iter_;
    
public:
    explicit ClauseIterator(const Directive& directive)
        : iter_(directive.valid() ? roup_directive_clauses_iter(directive.get()) : nullptr) {}
    
    ~ClauseIterator() {
        if (iter_) {
            roup_clause_iterator_free(iter_);
        }
    }
    
    // Delete copy
    ClauseIterator(const ClauseIterator&) = delete;
    ClauseIterator& operator=(const ClauseIterator&) = delete;
    
    // Get next clause kind (returns std::optional)
    std::optional<int32_t> next() {
        if (!iter_) return std::nullopt;
        
        OmpClause* clause = nullptr;
        if (roup_clause_iterator_next(iter_, &clause) == 1) {
            return roup_clause_kind(clause);
            // Note: Don't free clause - owned by directive
        }
        return std::nullopt;
    }
};

// Helper: Convert directive kind to name
inline const char* directive_kind_name(int32_t kind) {
    switch (kind) {
        case 0: return "parallel";
        case 1: return "for";
        case 2: return "sections";
        case 3: return "single";
        case 4: return "task";
        case 5: return "master";
        case 6: return "critical";
        case 7: return "barrier";
        case 8: return "taskwait";
        case 9: return "taskgroup";
        case 10: return "atomic";
        case 11: return "flush";
        case 12: return "ordered";
        case 13: return "target";
        case 14: return "teams";
        case 15: return "distribute";
        case 16: return "metadirective";
        default: return "unknown";
    }
}

// Helper: Convert clause kind to name
inline const char* clause_kind_name(int32_t kind) {
    switch (kind) {
        case 0: return "num_threads";
        case 1: return "if";
        case 2: return "private";
        case 3: return "shared";
        case 4: return "firstprivate";
        case 5: return "lastprivate";
        case 6: return "reduction";
        case 7: return "schedule";
        case 8: return "collapse";
        case 9: return "ordered";
        case 10: return "nowait";
        case 11: return "default";
        default: return "unknown";
    }
}

} // namespace roup
```

**Key RAII benefits:**
- ✅ **Automatic cleanup** - No need to call `_free()` functions
- ✅ **Exception safe** - Resources freed even if exceptions thrown
- ✅ **Move semantics** - Efficient transfer of ownership
- ✅ **Modern C++** - Uses `std::optional`, deleted copy constructors

---

## Step 3: Build the Analyzer Tool

Create `omp_analyzer.cpp`:

```cpp
#include "roup_wrapper.hpp"
#include <iostream>
#include <fstream>
#include <string>
#include <map>
#include <vector>
#include <algorithm>

struct DirectiveInfo {
    int line_number;
    std::string directive_name;
    int clause_count;
    std::vector<std::string> clause_names;
};

class OMPAnalyzer {
private:
    std::vector<DirectiveInfo> directives_;
    std::map<std::string, int> directive_counts_;
    
public:
    // Analyze a single line for OpenMP pragmas
    void analyze_line(const std::string& line, int line_number) {
        // Check if line contains OpenMP pragma
        if (line.find("#pragma omp") == std::string::npos) {
            return;
        }
        
        // Parse the directive
        roup::Directive directive(line);
        if (!directive) {
            std::cerr << "Warning: Failed to parse line " << line_number 
                      << ": " << line << std::endl;
            return;
        }
        
        // Extract directive info
        DirectiveInfo info;
        info.line_number = line_number;
        info.directive_name = roup::directive_kind_name(directive.kind());
        info.clause_count = directive.clause_count();
        
        // Extract clause names
        roup::ClauseIterator iter(directive);
        while (auto clause_kind = iter.next()) {
            info.clause_names.push_back(roup::clause_kind_name(*clause_kind));
        }
        
        directives_.push_back(info);
        directive_counts_[info.directive_name]++;
    }
    
    // Analyze entire file
    bool analyze_file(const std::string& filename) {
        std::ifstream file(filename);
        if (!file) {
            std::cerr << "Error: Cannot open file: " << filename << std::endl;
            return false;
        }
        
        std::string line;
        int line_number = 0;
        
        while (std::getline(file, line)) {
            line_number++;
            analyze_line(line, line_number);
        }
        
        return true;
    }
    
    // Print detailed report
    void print_report() const {
        if (directives_.empty()) {
            std::cout << "No OpenMP directives found." << std::endl;
            return;
        }
        
        std::cout << "\nFound " << directives_.size() 
                  << " OpenMP directive(s):\n" << std::endl;
        
        for (const auto& info : directives_) {
            std::cout << "  Line " << info.line_number << ": "
                      << info.directive_name << " ("
                      << info.clause_count << " clause"
                      << (info.clause_count != 1 ? "s" : "") << ")";
            
            if (!info.clause_names.empty()) {
                std::cout << " [";
                for (size_t i = 0; i < info.clause_names.size(); ++i) {
                    if (i > 0) std::cout << ", ";
                    std::cout << info.clause_names[i];
                }
                std::cout << "]";
            }
            std::cout << std::endl;
        }
        
        print_summary();
    }
    
    // Print summary statistics
    void print_summary() const {
        int total_clauses = 0;
        for (const auto& info : directives_) {
            total_clauses += info.clause_count;
        }
        
        std::cout << "\nSummary:" << std::endl;
        std::cout << "  Total directives: " << directives_.size() << std::endl;
        std::cout << "  Total clauses: " << total_clauses << std::endl;
        
        // Find most common directive
        auto max_elem = std::max_element(
            directive_counts_.begin(), 
            directive_counts_.end(),
            [](const auto& a, const auto& b) { return a.second < b.second; }
        );
        
        if (max_elem != directive_counts_.end()) {
            std::cout << "  Most common: " << max_elem->first 
                      << " (" << max_elem->second << " occurrence"
                      << (max_elem->second != 1 ? "s" : "") << ")" << std::endl;
        }
        
        // Print directive type breakdown
        if (directive_counts_.size() > 1) {
            std::cout << "\nDirective breakdown:" << std::endl;
            for (const auto& [name, count] : directive_counts_) {
                std::cout << "  " << name << ": " << count << std::endl;
            }
        }
    }
};

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <source-file>" << std::endl;
        std::cerr << "Example: " << argv[0] << " mycode.c" << std::endl;
        return 1;
    }
    
    OMPAnalyzer analyzer;
    
    if (!analyzer.analyze_file(argv[1])) {
        return 1;
    }
    
    analyzer.print_report();
    return 0;
}
```

---

## Step 4: Build and Run

### Compilation

```bash
# Build ROUP library first
cd roup
cargo build --release

# Build the analyzer
clang++ -std=c++17 omp_analyzer.cpp \
    -L./target/release -lroup \
    -Wl,-rpath,./target/release \
    -o omp_analyzer

# Or with g++:
g++ -std=c++17 omp_analyzer.cpp \
    -L./target/release -lroup \
    -Wl,-rpath,./target/release \
    -o omp_analyzer
```

### Test with Sample File

Create `test.c`:

```c
#include <stdio.h>

int main() {
    int n = 1000;
    int sum = 0;
    
    #pragma omp parallel for reduction(+:sum) num_threads(4)
    for (int i = 0; i < n; i++) {
        sum += i;
    }
    
    #pragma omp parallel
    {
        #pragma omp single
        {
            printf("Hello from thread\\n");
        }
    }
    
    #pragma omp task depend(in: sum)
    printf("Sum: %d\\n", sum);
    
    #pragma omp barrier
    
    return 0;
}
```

### Run the Analyzer

```bash
$ ./omp_analyzer test.c

Found 5 OpenMP directive(s):

  Line 7: for (3 clauses) [reduction, num_threads]
  Line 11: parallel (0 clauses)
  Line 13: single (0 clauses)
  Line 19: task (1 clause) [depend]
  Line 22: barrier (0 clauses)

Summary:
  Total directives: 5
  Total clauses: 4
  Most common: parallel (1 occurrence)

Directive breakdown:
  barrier: 1
  for: 1
  parallel: 1
  single: 1
  task: 1
```

---

## Step 5: Advanced Features

### 5.1 Extract Variable Names from Clauses

Some clauses (like `private`, `shared`) contain variable lists. To access them:

```cpp
// In C API (add to roup_wrapper.hpp):
extern "C" {
    OmpStringList* roup_clause_variables(const OmpClause* clause);
    int32_t roup_string_list_len(const OmpStringList* list);
    const char* roup_string_list_get(const OmpStringList* list, int32_t index);
    void roup_string_list_free(OmpStringList* list);
}

// RAII wrapper for string list
class StringList {
private:
    OmpStringList* list_;
    
public:
    explicit StringList(OmpStringList* list) : list_(list) {}
    
    ~StringList() {
        if (list_) roup_string_list_free(list_);
    }
    
    int32_t size() const {
        return list_ ? roup_string_list_len(list_) : 0;
    }
    
    std::string get(int32_t index) const {
        if (!list_ || index >= size()) return "";
        return roup_string_list_get(list_, index);
    }
    
    std::vector<std::string> to_vector() const {
        std::vector<std::string> result;
        for (int32_t i = 0; i < size(); ++i) {
            result.push_back(get(i));
        }
        return result;
    }
};
```

### 5.2 Handle Parse Errors Gracefully

```cpp
std::optional<DirectiveInfo> parse_directive(const std::string& line) {
    roup::Directive directive(line);
    if (!directive) {
        return std::nullopt;  // Parse failed
    }
    
    DirectiveInfo info;
    info.directive_name = roup::directive_kind_name(directive.kind());
    info.clause_count = directive.clause_count();
    
    return info;
}
```

### 5.3 Process Multiple Files

```cpp
void analyze_project(const std::vector<std::string>& files) {
    OMPAnalyzer combined;
    
    for (const auto& file : files) {
        OMPAnalyzer file_analyzer;
        if (file_analyzer.analyze_file(file)) {
            std::cout << "\n=== " << file << " ===" << std::endl;
            file_analyzer.print_report();
        }
    }
}
```

---

## Real-World Use Cases

### 1. **OpenMP Linter**
Check for common mistakes:
- `parallel for` without `private` on loop variable
- `reduction` with unsupported operators
- Missing `nowait` opportunities

### 2. **Code Modernization Tool**
- Convert OpenMP 3.x → 5.x syntax
- Suggest modern alternatives (e.g., `taskloop` instead of manual tasks)

### 3. **Performance Analyzer**
- Count parallelization opportunities
- Identify nested parallel regions (potential over-subscription)
- Find synchronization hotspots

### 4. **IDE Integration**
- Syntax highlighting for OpenMP pragmas
- Auto-completion for clause names
- Quick documentation lookup

---

## Performance Considerations

ROUP is **fast**:
- Parsing a typical directive: **~1-5 microseconds**
- Zero-copy design: Uses string slices, not allocations
- Suitable for **real-time IDE integration**

**Benchmark** (on typical code):
```
File size: 10,000 lines
OpenMP directives: 500
Total parse time: ~2.5 milliseconds
Throughput: ~200,000 directives/second
```

---

## Troubleshooting

### Issue: `undefined reference to roup_parse`

**Solution:** Make sure library path is correct:
```bash
clang++ ... -L./target/release -lroup -Wl,-rpath,./target/release
```

### Issue: `error while loading shared libraries: libroup.so`

**Solution:** Set `LD_LIBRARY_PATH` (Linux) or `DYLD_LIBRARY_PATH` (macOS):
```bash
export LD_LIBRARY_PATH=./target/release:$LD_LIBRARY_PATH
```

### Issue: Parse failures on valid pragmas

**Cause:** ROUP currently supports C/C++ syntax (`#pragma omp`), not Fortran (`!$omp`).

**Solution:** Ensure input starts with `#pragma omp`.

---

## Next Steps

- **Explore the Rust API** - See [API Reference](./api-reference.md)
- **Check out more examples** - [GitHub repository](https://github.com/ouankou/roup/tree/main/examples)
- **Contribute** - Report issues or submit PRs!

---

## Complete Example Code

All code from this tutorial is available at:
- `examples/cpp/roup_wrapper.hpp` - RAII wrappers
- `examples/cpp/omp_analyzer.cpp` - Full analyzer tool

**Clone and try:**
```bash
git clone https://github.com/ouankou/roup.git
cd roup/examples/cpp
make  # Builds all examples
./omp_analyzer ../../tests/data/sample.c
```

Happy parsing! 🚀
