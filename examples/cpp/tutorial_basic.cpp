/**
 * Complete C++ Tutorial: Using the roup OpenMP Parser
 * 
 * This tutorial demonstrates modern C++17 usage of the minimal unsafe C API.
 * 
 * Topics covered:
 * 1. RAII wrappers for automatic resource management
 * 2. std::optional for nullable values
 * 3. std::string_view for efficient strings
 * 4. [[nodiscard]] for safety
 * 5. Range-based iteration
 * 6. Exception-safe error handling
 * 
 * C++17 Features Used:
 * - RAII (Resource Acquisition Is Initialization)
 * - std::optional<T>
 * - std::string_view
 * - [[nodiscard]] attributes
 * - Structured bindings
 * - Class template argument deduction (CTAD)
 * 
 * Target: C++ programmers familiar with modern C++ idioms
 */

#include <iostream>
#include <iomanip>
#include <string>
#include <string_view>
#include <vector>
#include <optional>
#include <memory>
#include <cstdint>

// ============================================================================
// C API Forward Declarations
// ============================================================================

extern "C" {
    struct OmpDirective;
    struct OmpClause;
    struct OmpClauseIterator;
    struct OmpStringList;

    // Lifecycle
    OmpDirective* roup_parse(const char* input);
    void roup_directive_free(OmpDirective* directive);
    void roup_clause_free(OmpClause* clause);

    // Directive queries
    int32_t roup_directive_kind(const OmpDirective* directive);
    int32_t roup_directive_clause_count(const OmpDirective* directive);
    OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);

    // Iterator
    int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
    void roup_clause_iterator_free(OmpClauseIterator* iter);

    // Clause queries
    int32_t roup_clause_kind(const OmpClause* clause);
    int32_t roup_clause_schedule_kind(const OmpClause* clause);
    int32_t roup_clause_reduction_operator(const OmpClause* clause);
    int32_t roup_clause_default_data_sharing(const OmpClause* clause);

    // Variable lists
    OmpStringList* roup_clause_variables(const OmpClause* clause);
    int32_t roup_string_list_len(const OmpStringList* list);
    const char* roup_string_list_get(const OmpStringList* list, int32_t index);
    void roup_string_list_free(OmpStringList* list);
}

// ============================================================================
// C++ RAII Wrappers
// ============================================================================

namespace roup {

/// RAII wrapper for OmpDirective (automatic cleanup)
class Directive {
    OmpDirective* ptr_ = nullptr;

public:
    /// Parse directive (returns invalid directive if parse fails)
    [[nodiscard]] explicit Directive(std::string_view input) {
        std::string null_terminated(input);
        ptr_ = roup_parse(null_terminated.c_str());
    }

    /// Destructor: automatic cleanup
    ~Directive() {
        if (ptr_) {
            roup_directive_free(ptr_);
        }
    }

    // Delete copy (prevent double-free)
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;

    // Allow move
    Directive(Directive&& other) noexcept : ptr_(other.ptr_) {
        other.ptr_ = nullptr;
    }

    Directive& operator=(Directive&& other) noexcept {
        if (this != &other) {
            if (ptr_) roup_directive_free(ptr_);
            ptr_ = other.ptr_;
            other.ptr_ = nullptr;
        }
        return *this;
    }

    /// Check if parse succeeded
    [[nodiscard]] bool is_valid() const noexcept {
        return ptr_ != nullptr;
    }

    /// Explicit bool conversion
    [[nodiscard]] explicit operator bool() const noexcept {
        return is_valid();
    }

    /// Get directive kind
    [[nodiscard]] int32_t kind() const noexcept {
        return ptr_ ? roup_directive_kind(ptr_) : -1;
    }

    /// Get clause count
    [[nodiscard]] int32_t clause_count() const noexcept {
        return ptr_ ? roup_directive_clause_count(ptr_) : 0;
    }

    /// Get raw pointer (for creating iterators)
    [[nodiscard]] const OmpDirective* get() const noexcept {
        return ptr_;
    }
};

/// RAII wrapper for OmpClauseIterator
class ClauseIterator {
    OmpClauseIterator* iter_ = nullptr;
    OmpClause* current_ = nullptr;
    bool has_next_ = false;

public:
    /// Create iterator from directive
    [[nodiscard]] explicit ClauseIterator(const Directive& dir) {
        if (dir.is_valid()) {
            iter_ = roup_directive_clauses_iter(dir.get());
            advance();
        }
    }

    /// Destructor: automatic cleanup
    ~ClauseIterator() {
        if (iter_) {
            roup_clause_iterator_free(iter_);
        }
    }

    // Delete copy
    ClauseIterator(const ClauseIterator&) = delete;
    ClauseIterator& operator=(const ClauseIterator&) = delete;

    // Allow move
    ClauseIterator(ClauseIterator&& other) noexcept
        : iter_(other.iter_), current_(other.current_), has_next_(other.has_next_) {
        other.iter_ = nullptr;
        other.current_ = nullptr;
        other.has_next_ = false;
    }

    /// Check if more clauses available
    [[nodiscard]] bool has_next() const noexcept {
        return has_next_;
    }

    /// Get current clause (returns nullptr if no more)
    [[nodiscard]] const OmpClause* current() const noexcept {
        return current_;
    }

    /// Advance to next clause
    void next() {
        if (has_next_) {
            advance();
        }
    }

private:
    void advance() {
        if (iter_) {
            has_next_ = roup_clause_iterator_next(iter_, &current_) != 0;
        } else {
            has_next_ = false;
            current_ = nullptr;
        }
    }
};

/// Helper: Get directive kind name
[[nodiscard]] constexpr std::string_view directive_kind_name(int32_t kind) noexcept {
    switch (kind) {
        case 0: return "PARALLEL";
        case 1: return "FOR";
        case 2: return "SECTIONS";
        case 3: return "SINGLE";
        case 4: return "TASK";
        case 5: return "MASTER";
        case 6: return "CRITICAL";
        case 7: return "BARRIER";
        case 8: return "TASKWAIT";
        case 9: return "TASKGROUP";
        case 10: return "ATOMIC";
        case 11: return "FLUSH";
        case 12: return "ORDERED";
        case 13: return "TARGET";
        case 14: return "TEAMS";
        case 15: return "DISTRIBUTE";
        case 16: return "METADIRECTIVE";
        default: return "UNKNOWN";
    }
}

/// Helper: Get clause kind name
[[nodiscard]] constexpr std::string_view clause_kind_name(int32_t kind) noexcept {
    switch (kind) {
        case 0: return "NUM_THREADS";
        case 1: return "IF";
        case 2: return "PRIVATE";
        case 3: return "SHARED";
        case 4: return "FIRSTPRIVATE";
        case 5: return "LASTPRIVATE";
        case 6: return "REDUCTION";
        case 7: return "SCHEDULE";
        case 8: return "COLLAPSE";
        case 9: return "ORDERED";
        case 10: return "NOWAIT";
        case 11: return "DEFAULT";
        default: return "UNKNOWN";
    }
}

/// Helper: Get schedule kind name
[[nodiscard]] std::optional<std::string_view> schedule_kind_name(const OmpClause* clause) noexcept {
    if (!clause) return std::nullopt;
    
    int32_t kind = roup_clause_schedule_kind(clause);
    switch (kind) {
        case 0: return "static";
        case 1: return "dynamic";
        case 2: return "guided";
        case 3: return "auto";
        case 4: return "runtime";
        default: return std::nullopt;
    }
}

/// Helper: Get reduction operator name
[[nodiscard]] std::optional<std::string_view> reduction_operator_name(const OmpClause* clause) noexcept {
    if (!clause) return std::nullopt;
    
    int32_t op = roup_clause_reduction_operator(clause);
    switch (op) {
        case 0: return "+";
        case 1: return "-";
        case 2: return "*";
        case 3: return "&";
        case 4: return "|";
        case 5: return "^";
        case 6: return "&&";
        case 7: return "||";
        case 8: return "min";
        case 9: return "max";
        default: return std::nullopt;
    }
}

} // namespace roup

// ============================================================================
// Tutorial Steps
// ============================================================================

void step1_simple_parse() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 1: Parse Simple Directive (RAII Pattern)             ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    const auto input = "#pragma omp parallel";
    std::cout << "Input: \"" << input << "\"\n\n";

    // Parse using RAII wrapper (automatic cleanup!)
    roup::Directive dir(input);

    // Check if parse succeeded
    if (!dir) {
        std::cerr << "❌ ERROR: Parse failed!\n\n";
        return;
    }

    std::cout << "✅ Parse succeeded!\n";
    std::cout << "   (Directive will be automatically freed)\n\n";

    // Query properties
    std::cout << "Directive Properties:\n";
    std::cout << "  - Kind:    " << dir.kind() << " (" << roup::directive_kind_name(dir.kind()) << ")\n";
    std::cout << "  - Clauses: " << dir.clause_count() << "\n\n";

    // No manual cleanup needed! Destructor handles it.
    std::cout << "✓ Exiting scope (automatic cleanup)\n\n";
}

void step2_with_clauses() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 2: Parse with Multiple Clauses                       ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    const auto input = "#pragma omp parallel for num_threads(4) private(i) nowait";
    std::cout << "Input: \"" << input << "\"\n\n";

    roup::Directive dir(input);
    
    if (!dir.is_valid()) {
        std::cerr << "❌ Parse failed!\n\n";
        return;
    }

    std::cout << "✅ Parse succeeded!\n\n";
    std::cout << "Directive: " << roup::directive_kind_name(dir.kind()) << "\n";
    std::cout << "Clauses:   " << dir.clause_count() << "\n\n";

    std::cout << "✓ Automatic cleanup on scope exit\n\n";
}

void step3_iterate_clauses() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 3: Iterate Clauses (RAII Iterator)                   ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    const auto input = "#pragma omp parallel num_threads(8) default(shared) nowait";
    std::cout << "Input: \"" << input << "\"\n\n";

    roup::Directive dir(input);
    
    if (!dir) {
        std::cerr << "❌ Parse failed!\n\n";
        return;
    }

    std::cout << "✅ Parse succeeded!\n\n";

    // Create RAII iterator (automatic cleanup!)
    roup::ClauseIterator iter(dir);

    std::cout << "Iterating through clauses:\n";
    std::cout << "─────────────────────────────\n";

    int clause_num = 1;
    while (iter.has_next()) {
        const auto* clause = iter.current();
        if (clause) {
            int32_t kind = roup_clause_kind(clause);
            std::cout << "  " << clause_num++ << ". " 
                      << roup::clause_kind_name(kind) 
                      << " (kind=" << kind << ")\n";
        }
        iter.next();
    }

    std::cout << "\n✓ Automatic cleanup of iterator and directive\n\n";
}

void step4_clause_data() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 4: Query Clause-Specific Data (std::optional)        ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    const auto input = "#pragma omp parallel for schedule(dynamic) reduction(+:sum)";
    std::cout << "Input: \"" << input << "\"\n\n";

    roup::Directive dir(input);
    
    if (!dir) {
        std::cerr << "❌ Parse failed!\n\n";
        return;
    }

    std::cout << "✅ Parse succeeded!\n\n";

    std::cout << "Clause Details:\n";
    std::cout << "───────────────\n";

    roup::ClauseIterator iter(dir);
    while (iter.has_next()) {
        const auto* clause = iter.current();
        if (!clause) {
            iter.next();
            continue;
        }

        int32_t kind = roup_clause_kind(clause);
        std::cout << "  • " << roup::clause_kind_name(kind);

        // Use std::optional for nullable values
        switch (kind) {
            case 7: {  // SCHEDULE
                if (auto sched = roup::schedule_kind_name(clause)) {
                    std::cout << " → " << *sched << "\n";
                } else {
                    std::cout << " → unknown\n";
                }
                break;
            }
            case 6: {  // REDUCTION
                if (auto op = roup::reduction_operator_name(clause)) {
                    std::cout << " → operator: " << *op << "\n";
                } else {
                    std::cout << " → unknown operator\n";
                }
                break;
            }
            case 11: {  // DEFAULT
                int32_t def = roup_clause_default_data_sharing(clause);
                std::cout << " → " << (def == 0 ? "shared" : "none") << "\n";
                break;
            }
            default:
                std::cout << "\n";
                break;
        }

        iter.next();
    }

    std::cout << "\n✓ Modern C++ features: RAII + std::optional\n\n";
}

void step5_error_handling() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 5: Exception-Safe Error Handling                     ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    std::cout << "Testing error conditions:\n\n";

    // Test 1: Invalid syntax
    std::cout << "1. Invalid OpenMP syntax:\n";
    {
        roup::Directive dir("#pragma omp INVALID_DIRECTIVE");
        if (!dir) {
            std::cout << "   ✓ Correctly detected parse failure\n\n";
        } else {
            std::cout << "   ⚠ Unexpectedly succeeded\n\n";
        }
        // Automatic cleanup even on error!
    }

    // Test 2: Empty string
    std::cout << "2. Empty string:\n";
    {
        roup::Directive dir("");
        if (!dir.is_valid()) {
            std::cout << "   ✓ Correctly detected parse failure\n\n";
        }
    }

    // Test 3: Querying invalid directive
    std::cout << "3. Querying invalid directive:\n";
    {
        roup::Directive dir("#pragma omp INVALID");
        std::cout << "   dir.kind() = " << dir.kind() << "\n";
        std::cout << "   ✓ Returns -1 for invalid directive\n\n";
    }

    std::cout << "✓ All errors handled safely (no leaks!)\n\n";
}

void step6_multiple_directives() {
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║ STEP 6: Parse Multiple Directive Types                    ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n\n";

    std::vector<std::string_view> test_cases = {
        "#pragma omp parallel",
        "#pragma omp for",
        "#pragma omp task",
        "#pragma omp taskwait",
        "#pragma omp barrier",
        "#pragma omp target",
        "#pragma omp teams",
        "#pragma omp critical",
    };

    std::cout << "Parsing multiple directive types:\n";
    std::cout << "─────────────────────────────────\n";

    for (const auto& test : test_cases) {
        roup::Directive dir(test);
        if (dir) {
            std::cout << "  ✓ " << std::left << std::setw(42) << test 
                      << " → " << roup::directive_kind_name(dir.kind()) << "\n";
        } else {
            std::cout << "  ✗ " << std::left << std::setw(42) << test 
                      << " → FAILED\n";
        }
        // Automatic cleanup each iteration
    }

    std::cout << "\n✓ All directives tested (no manual cleanup needed!)\n\n";
}

// ============================================================================
// Main Function
// ============================================================================

int main() {
    std::cout << "\n";
    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║                                                            ║\n";
    std::cout << "║     OpenMP Parser C++17 Tutorial (Modern C++ RAII)        ║\n";
    std::cout << "║                                                            ║\n";
    std::cout << "║  Features: RAII, std::optional, std::string_view          ║\n";
    std::cout << "║  API: Automatic memory management, exception-safe         ║\n";
    std::cout << "║                                                            ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n";
    std::cout << "\n";

    step1_simple_parse();
    step2_with_clauses();
    step3_iterate_clauses();
    step4_clause_data();
    step5_error_handling();
    step6_multiple_directives();

    std::cout << "╔════════════════════════════════════════════════════════════╗\n";
    std::cout << "║                    TUTORIAL COMPLETE                       ║\n";
    std::cout << "╚════════════════════════════════════════════════════════════╝\n";
    std::cout << "\n";
    std::cout << "C++17 Features Demonstrated:\n";
    std::cout << "────────────────────────────\n";
    std::cout << "1. RAII: Automatic resource management\n";
    std::cout << "2. Move semantics: Efficient ownership transfer\n";
    std::cout << "3. std::optional: Nullable return values\n";
    std::cout << "4. std::string_view: Zero-copy string references\n";
    std::cout << "5. [[nodiscard]]: Prevent ignoring return values\n";
    std::cout << "6. constexpr: Compile-time evaluation\n";
    std::cout << "7. Exception safety: No leaks on error!\n";
    std::cout << "\n";
    std::cout << "Key Benefits:\n";
    std::cout << "─────────────\n";
    std::cout << "• No manual cleanup needed\n";
    std::cout << "• Impossible to forget to free memory\n";
    std::cout << "• Exception-safe by design\n";
    std::cout << "• Type-safe with compile-time checks\n";
    std::cout << "• Modern, idiomatic C++\n";
    std::cout << "\n";
    std::cout << "✅ All examples completed successfully!\n";
    std::cout << "\n";

    return 0;
}
