/**
 * @file roup.h
 * @brief Roup OpenMP Parser - C FFI Header
 * 
 * A 100% safe Rust-based OpenMP parser with a complete C FFI.
 * This library provides zero-copy parsing of OpenMP pragmas/directives
 * with comprehensive query APIs for directive and clause inspection.
 * 
 * Memory Safety Guarantees:
 * - All resources are managed through opaque handles (uint64_t)
 * - No raw pointers exposed to C code
 * - Thread-safe global registry with automatic cleanup
 * - All strings must be explicitly freed with omp_str_free()
 * - All parse results must be freed with omp_parse_result_free()
 * - All clauses must be freed with omp_clause_free()
 * 
 * Basic Usage Pattern:
 * 1. Parse input: omp_parse(input, lang) -> parse_result_handle
 * 2. Extract directives: omp_take_last_parse_result(&directives, &count)
 * 3. Query directives: omp_directive_kind(), omp_directive_clause_count()
 * 4. Query clauses: omp_clause_at(), omp_clause_type(), typed accessors
 * 5. Clean up: omp_clause_free(), omp_parse_result_free(), omp_str_free()
 * 
 * Error Handling:
 * - All functions return OmpStatus (Success = 0, errors > 0)
 * - Use INVALID_HANDLE (0) to check for invalid handles
 * - Check status codes after each operation
 * 
 * @version 0.1.0
 * @author Roup Contributors
 * @license MIT/Apache-2.0
 */

#ifndef ROUP_H
#define ROUP_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Type Definitions
 * ========================================================================= */

/**
 * @brief Opaque handle type for all Roup resources
 * 
 * Handles are used to reference strings, parse results, directives, clauses,
 * and cursors. All handles must be freed with the appropriate free function.
 * A handle value of 0 (INVALID_HANDLE) indicates an invalid or null resource.
 */
typedef uint64_t Handle;

/**
 * @brief Invalid handle constant
 * 
 * Used to indicate errors or missing resources. Always check if a returned
 * handle equals INVALID_HANDLE before using it.
 */
#define INVALID_HANDLE ((Handle)0)

/* ============================================================================
 * Enumerations
 * ========================================================================= */

/**
 * @brief Status codes returned by all FFI functions
 * 
 * Success is always 0, all error codes are positive integers.
 * Always check the return status before using output parameters.
 */
typedef enum {
    OMP_SUCCESS = 0,           /**< Operation completed successfully */
    OMP_INVALID_HANDLE = 1,    /**< Handle not found in registry */
    OMP_INVALID_UTF8 = 2,      /**< String contains invalid UTF-8 */
    OMP_NULL_POINTER = 3,      /**< Required pointer parameter is NULL */
    OMP_OUT_OF_BOUNDS = 4,     /**< Index exceeds array/collection bounds */
    OMP_PARSE_ERROR = 5,       /**< Failed to parse OpenMP directive */
    OMP_TYPE_MISMATCH = 6,     /**< Clause type doesn't match expected type */
    OMP_EMPTY_RESULT = 7,      /**< Operation returned no results */
} OmpStatus;

/**
 * @brief Programming language for parsing context
 * 
 * Determines whether to expect C-style (#pragma) or Fortran-style (!$omp)
 * directive syntax.
 */
typedef enum {
    OMP_LANG_C = 0,       /**< C/C++ language (#pragma omp) */
    OMP_LANG_FORTRAN = 1, /**< Fortran language (!$omp, !$OMP, c$omp, *$omp) */
} Language;

/**
 * @brief OpenMP directive kinds
 * 
 * Represents all supported OpenMP directive types. Use omp_directive_kind()
 * to query the kind of a directive handle.
 */
typedef enum {
    OMP_DIR_PARALLEL = 0,
    OMP_DIR_FOR = 1,
    OMP_DIR_SECTIONS = 2,
    OMP_DIR_SECTION = 3,
    OMP_DIR_SINGLE = 4,
    OMP_DIR_TASK = 5,
    OMP_DIR_MASTER = 6,
    OMP_DIR_CRITICAL = 7,
    OMP_DIR_BARRIER = 8,
    OMP_DIR_TASKWAIT = 9,
    OMP_DIR_TASKGROUP = 10,
    OMP_DIR_ATOMIC = 11,
    OMP_DIR_FLUSH = 12,
    OMP_DIR_ORDERED = 13,
    OMP_DIR_SIMD = 14,
    OMP_DIR_TARGET = 15,
    OMP_DIR_TARGET_DATA = 16,
    OMP_DIR_TARGET_ENTER_DATA = 17,
    OMP_DIR_TARGET_EXIT_DATA = 18,
    OMP_DIR_TARGET_UPDATE = 19,
    OMP_DIR_DECLARE_TARGET = 20,
    OMP_DIR_TEAMS = 21,
    OMP_DIR_DISTRIBUTE = 22,
    OMP_DIR_DECLARE_SIMD = 23,
    OMP_DIR_DECLARE_REDUCTION = 24,
    OMP_DIR_TASKLOOP = 25,
    OMP_DIR_CANCEL = 26,
    OMP_DIR_CANCELLATION_POINT = 27,
    OMP_DIR_PARALLEL_FOR = 28,
    OMP_DIR_PARALLEL_SECTIONS = 29,
    OMP_DIR_PARALLEL_MASTER = 30,
    OMP_DIR_MASTER_TASKLOOP = 31,
    OMP_DIR_PARALLEL_MASTER_TASKLOOP = 32,
    OMP_DIR_TARGET_PARALLEL = 33,
    OMP_DIR_TARGET_PARALLEL_FOR = 34,
    OMP_DIR_TARGET_SIMD = 35,
    OMP_DIR_TARGET_TEAMS = 36,
    OMP_DIR_TEAMS_DISTRIBUTE = 37,
    OMP_DIR_TEAMS_DISTRIBUTE_SIMD = 38,
    OMP_DIR_TARGET_TEAMS_DISTRIBUTE = 39,
    OMP_DIR_TARGET_TEAMS_DISTRIBUTE_SIMD = 40,
    OMP_DIR_DISTRIBUTE_PARALLEL_FOR = 41,
    OMP_DIR_DISTRIBUTE_PARALLEL_FOR_SIMD = 42,
    OMP_DIR_DISTRIBUTE_SIMD = 43,
    OMP_DIR_PARALLEL_FOR_SIMD = 44,
    OMP_DIR_TASKLOOP_SIMD = 45,
    OMP_DIR_MASTER_TASKLOOP_SIMD = 46,
    OMP_DIR_PARALLEL_MASTER_TASKLOOP_SIMD = 47,
    OMP_DIR_TARGET_PARALLEL_FOR_SIMD = 48,
    OMP_DIR_TEAMS_DISTRIBUTE_PARALLEL_FOR = 49,
    OMP_DIR_TEAMS_DISTRIBUTE_PARALLEL_FOR_SIMD = 50,
    OMP_DIR_TARGET_TEAMS_DISTRIBUTE_PARALLEL_FOR = 51,
    OMP_DIR_TARGET_TEAMS_DISTRIBUTE_PARALLEL_FOR_SIMD = 52,
    OMP_DIR_LOOP = 53,
    OMP_DIR_PARALLEL_LOOP = 54,
    OMP_DIR_TEAMS_LOOP = 55,
    OMP_DIR_TARGET_LOOP = 56,
    OMP_DIR_TARGET_PARALLEL_LOOP = 57,
    OMP_DIR_TARGET_TEAMS_LOOP = 58,
    OMP_DIR_MASKED = 59,
    OMP_DIR_SCOPE = 60,
    OMP_DIR_METADIRECTIVE = 61,
    OMP_DIR_DECLARE_VARIANT = 62,
    OMP_DIR_REQUIRES = 63,
    OMP_DIR_ASSUME = 64,
    OMP_DIR_NOTHING = 65,
    OMP_DIR_ERROR = 66,
    OMP_DIR_SCAN = 67,
    OMP_DIR_DEPOBJ = 68,
    OMP_DIR_TILE = 69,
    OMP_DIR_UNROLL = 70,
    OMP_DIR_ALLOCATE = 71,
    OMP_DIR_THREADPRIVATE = 72,
    OMP_DIR_DECLARE_MAPPER = 73,
} DirectiveKind;

/**
 * @brief OpenMP clause types
 * 
 * Represents all supported clause types. Use omp_clause_type() to query
 * the type of a clause handle, then use the appropriate typed accessor.
 */
typedef enum {
    OMP_CLAUSE_IF = 0,
    OMP_CLAUSE_NUM_THREADS = 1,
    OMP_CLAUSE_DEFAULT = 2,
    OMP_CLAUSE_PRIVATE = 3,
    OMP_CLAUSE_FIRSTPRIVATE = 4,
    OMP_CLAUSE_LASTPRIVATE = 5,
    OMP_CLAUSE_SHARED = 6,
    OMP_CLAUSE_REDUCTION = 7,
    OMP_CLAUSE_COPYIN = 8,
    OMP_CLAUSE_COPYPRIVATE = 9,
    OMP_CLAUSE_SCHEDULE = 10,
    OMP_CLAUSE_ORDERED = 11,
    OMP_CLAUSE_NOWAIT = 12,
    OMP_CLAUSE_COLLAPSE = 13,
    OMP_CLAUSE_UNTIED = 14,
    OMP_CLAUSE_FINAL = 15,
    OMP_CLAUSE_MERGEABLE = 16,
    OMP_CLAUSE_DEPEND = 17,
    OMP_CLAUSE_PRIORITY = 18,
    OMP_CLAUSE_GRAINSIZE = 19,
    OMP_CLAUSE_NUM_TASKS = 20,
    OMP_CLAUSE_NOGROUP = 21,
    OMP_CLAUSE_THREADS = 22,
    OMP_CLAUSE_SIMD = 23,
    OMP_CLAUSE_ALIGNED = 24,
    OMP_CLAUSE_LINEAR = 25,
    OMP_CLAUSE_UNIFORM = 26,
    OMP_CLAUSE_INBRANCH = 27,
    OMP_CLAUSE_NOTINBRANCH = 28,
    OMP_CLAUSE_SAFELEN = 29,
    OMP_CLAUSE_SIMDLEN = 30,
    OMP_CLAUSE_DEVICE = 31,
    OMP_CLAUSE_MAP = 32,
    OMP_CLAUSE_NUM_TEAMS = 33,
    OMP_CLAUSE_THREAD_LIMIT = 34,
    OMP_CLAUSE_DIST_SCHEDULE = 35,
    OMP_CLAUSE_PROC_BIND = 36,
    OMP_CLAUSE_DEFAULTMAP = 37,
    OMP_CLAUSE_TO = 38,
    OMP_CLAUSE_FROM = 39,
    OMP_CLAUSE_USE_DEVICE_PTR = 40,
    OMP_CLAUSE_IS_DEVICE_PTR = 41,
    OMP_CLAUSE_LINK = 42,
    OMP_CLAUSE_NONTEMPORAL = 43,
    OMP_CLAUSE_ORDER = 44,
    OMP_CLAUSE_DESTROY = 45,
    OMP_CLAUSE_DETACH = 46,
    OMP_CLAUSE_AFFINITY = 47,
    OMP_CLAUSE_BIND = 48,
    OMP_CLAUSE_FILTER = 49,
    OMP_CLAUSE_ALLOCATE = 50,
    OMP_CLAUSE_ALLOCATOR = 51,
    OMP_CLAUSE_USES_ALLOCATORS = 52,
    OMP_CLAUSE_INCLUSIVE = 53,
    OMP_CLAUSE_EXCLUSIVE = 54,
    OMP_CLAUSE_WHEN = 55,
    OMP_CLAUSE_MATCH = 56,
    OMP_CLAUSE_AT = 57,
    OMP_CLAUSE_SEVERITY = 58,
    OMP_CLAUSE_MESSAGE = 59,
    OMP_CLAUSE_NOVARIANTS = 60,
    OMP_CLAUSE_NOCONTEXT = 61,
    OMP_CLAUSE_ADJUST_ARGS = 62,
    OMP_CLAUSE_APPEND_ARGS = 63,
    OMP_CLAUSE_FULL = 64,
    OMP_CLAUSE_PARTIAL = 65,
    OMP_CLAUSE_SIZES = 66,
    OMP_CLAUSE_HOLDS = 67,
    OMP_CLAUSE_ABSENT = 68,
    OMP_CLAUSE_CONTAINS = 69,
    OMP_CLAUSE_ATOMIC_DEFAULT_MEM_ORDER = 70,
    OMP_CLAUSE_DYNAMIC_ALLOCATORS = 71,
    OMP_CLAUSE_REVERSE_OFFLOAD = 72,
    OMP_CLAUSE_UNIFIED_ADDRESS = 73,
    OMP_CLAUSE_UNIFIED_SHARED_MEMORY = 74,
    OMP_CLAUSE_COMPARE = 75,
    OMP_CLAUSE_FAIL = 76,
    OMP_CLAUSE_SEQ_CST = 77,
    OMP_CLAUSE_ACQ_REL = 78,
    OMP_CLAUSE_RELEASE = 79,
    OMP_CLAUSE_ACQUIRE = 80,
    OMP_CLAUSE_RELAXED = 81,
    OMP_CLAUSE_HINT = 82,
    OMP_CLAUSE_UPDATE = 83,
    OMP_CLAUSE_CAPTURE = 84,
    OMP_CLAUSE_READ = 85,
    OMP_CLAUSE_WRITE = 86,
    OMP_CLAUSE_INIT = 87,
    OMP_CLAUSE_USE_DEVICE_ADDR = 88,
    OMP_CLAUSE_HAS_DEVICE_ADDR = 89,
    OMP_CLAUSE_ENTER = 90,
    OMP_CLAUSE_DOACROSS = 91,
} ClauseType;

/**
 * @brief Schedule kind for schedule clause
 */
typedef enum {
    OMP_SCHEDULE_STATIC = 0,
    OMP_SCHEDULE_DYNAMIC = 1,
    OMP_SCHEDULE_GUIDED = 2,
    OMP_SCHEDULE_AUTO = 3,
    OMP_SCHEDULE_RUNTIME = 4,
} ScheduleKind;

/**
 * @brief Default data-sharing attribute
 */
typedef enum {
    OMP_DEFAULT_SHARED = 0,
    OMP_DEFAULT_NONE = 1,
    OMP_DEFAULT_PRIVATE = 2,
    OMP_DEFAULT_FIRSTPRIVATE = 3,
} DefaultKind;

/**
 * @brief Reduction operator
 */
typedef enum {
    OMP_REDUCTION_ADD = 0,      /**< + */
    OMP_REDUCTION_MULTIPLY = 1, /**< * */
    OMP_REDUCTION_SUBTRACT = 2, /**< - */
    OMP_REDUCTION_AND = 3,      /**< & */
    OMP_REDUCTION_OR = 4,       /**< | */
    OMP_REDUCTION_XOR = 5,      /**< ^ */
    OMP_REDUCTION_LAND = 6,     /**< && */
    OMP_REDUCTION_LOR = 7,      /**< || */
    OMP_REDUCTION_MIN = 8,      /**< min */
    OMP_REDUCTION_MAX = 9,      /**< max */
    OMP_REDUCTION_CUSTOM = 10,  /**< Custom identifier */
} ReductionOperator;

/* ============================================================================
 * String API (10 functions)
 * ========================================================================= */

/**
 * @brief Create a new empty string builder
 * @param[out] out_handle Handle to the created string (INVALID_HANDLE on error)
 * @return OMP_SUCCESS or error code
 * 
 * Example:
 *   Handle str;
 *   if (omp_str_new(&str) == OMP_SUCCESS) {
 *       // Use str...
 *       omp_str_free(str);
 *   }
 */
OmpStatus omp_str_new(Handle *out_handle);

/**
 * @brief Create a string from a C string
 * @param[in] c_str Null-terminated C string
 * @param[out] out_handle Handle to the created string
 * @return OMP_SUCCESS, OMP_NULL_POINTER, or OMP_INVALID_UTF8
 */
OmpStatus omp_str_from_cstr(const char *c_str, Handle *out_handle);

/**
 * @brief Append bytes to a string
 * @param[in] handle String handle
 * @param[in] data Bytes to append
 * @param[in] len Number of bytes
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_INVALID_UTF8
 */
OmpStatus omp_str_push_bytes(Handle handle, const uint8_t *data, uintptr_t len);

/**
 * @brief Append a C string to a string
 * @param[in] handle String handle
 * @param[in] c_str Null-terminated C string to append
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_INVALID_UTF8
 */
OmpStatus omp_str_push_cstr(Handle handle, const char *c_str);

/**
 * @brief Get the length of a string in bytes
 * @param[in] handle String handle
 * @param[out] out_len Length in bytes
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_str_len(Handle handle, uintptr_t *out_len);

/**
 * @brief Get the capacity of a string in bytes
 * @param[in] handle String handle
 * @param[out] out_capacity Capacity in bytes
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_str_capacity(Handle handle, uintptr_t *out_capacity);

/**
 * @brief Copy string contents to a C buffer
 * @param[in] handle String handle
 * @param[out] buffer Output buffer (must have space for len + 1 bytes)
 * @param[in] buffer_size Size of output buffer
 * @param[out] out_len Number of bytes written (excluding null terminator)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_OUT_OF_BOUNDS
 * 
 * The output buffer will be null-terminated. Use omp_str_len() to determine
 * the required buffer size.
 */
OmpStatus omp_str_copy_to_buffer(Handle handle, char *buffer, uintptr_t buffer_size, uintptr_t *out_len);

/**
 * @brief Clear a string (length becomes 0, capacity unchanged)
 * @param[in] handle String handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_str_clear(Handle handle);

/**
 * @brief Check if a string is empty
 * @param[in] handle String handle
 * @param[out] out_is_empty true if empty, false otherwise
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_str_is_empty(Handle handle, bool *out_is_empty);

/**
 * @brief Free a string and remove from registry
 * @param[in] handle String handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 * 
 * After calling this function, the handle becomes invalid.
 */
OmpStatus omp_str_free(Handle handle);

/* ============================================================================
 * Parser API (3 functions)
 * ========================================================================= */

/**
 * @brief Parse OpenMP directives from input text
 * @param[in] input Input text containing OpenMP directives
 * @param[in] lang Programming language (C or Fortran)
 * @param[out] out_handle Handle to parse result (array of directives)
 * @return OMP_SUCCESS, OMP_NULL_POINTER, OMP_INVALID_UTF8, or OMP_PARSE_ERROR
 * 
 * The parse result contains an array of directive handles. Use
 * omp_take_last_parse_result() to extract them.
 * 
 * Example:
 *   Handle result;
 *   if (omp_parse("#pragma omp parallel num_threads(4)", OMP_LANG_C, &result) == OMP_SUCCESS) {
 *       Handle *directives;
 *       uintptr_t count;
 *       omp_take_last_parse_result(&directives, &count);
 *       // Use directives...
 *       omp_parse_result_free(result);
 *   }
 */
OmpStatus omp_parse(const char *input, Language lang, Handle *out_handle);

/**
 * @brief Extract directive handles from last parse result
 * @param[out] out_directives Pointer to array of directive handles (caller must free)
 * @param[out] out_count Number of directives in array
 * @return OMP_SUCCESS, OMP_NULL_POINTER, or OMP_EMPTY_RESULT
 * 
 * This function returns a heap-allocated array of handles. The caller must
 * free this array with free() when done. Each directive handle must be
 * individually freed with omp_directive_free() (if needed) or will be cleaned
 * up when the parse result is freed.
 */
OmpStatus omp_take_last_parse_result(Handle **out_directives, uintptr_t *out_count);

/**
 * @brief Free a parse result and all associated directives
 * @param[in] handle Parse result handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_parse_result_free(Handle handle);

/* ============================================================================
 * Directive Query API (14 functions)
 * ========================================================================= */

/**
 * @brief Get the kind of a directive
 * @param[in] handle Directive handle
 * @param[out] out_kind Directive kind
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_directive_kind(Handle handle, DirectiveKind *out_kind);

/**
 * @brief Get the number of clauses in a directive
 * @param[in] handle Directive handle
 * @param[out] out_count Number of clauses
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_directive_clause_count(Handle handle, uintptr_t *out_count);

/**
 * @brief Get the source line number of a directive
 * @param[in] handle Directive handle
 * @param[out] out_line Line number (1-indexed)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_directive_line(Handle handle, uintptr_t *out_line);

/**
 * @brief Get the source column number of a directive
 * @param[in] handle Directive handle
 * @param[out] out_column Column number (0-indexed)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_directive_column(Handle handle, uintptr_t *out_column);

/**
 * @brief Get the programming language of a directive
 * @param[in] handle Directive handle
 * @param[out] out_lang Language (C or Fortran)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_directive_language(Handle handle, Language *out_lang);

/**
 * @brief Create a cursor for iterating over directive clauses
 * @param[in] handle Directive handle
 * @param[out] out_cursor Cursor handle
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 * 
 * Cursors provide an iterator pattern for traversing clauses.
 * Use omp_cursor_next() to advance and omp_cursor_current() to get the
 * current clause. Free with omp_cursor_free() when done.
 */
OmpStatus omp_directive_clauses_cursor(Handle handle, Handle *out_cursor);

/**
 * @brief Move cursor to next clause
 * @param[in] handle Cursor handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 * 
 * Advances the cursor position. Use omp_cursor_is_done() to check if
 * the cursor has reached the end.
 */
OmpStatus omp_cursor_next(Handle handle);

/**
 * @brief Get the current clause from cursor
 * @param[in] handle Cursor handle
 * @param[out] out_clause Clause handle (INVALID_HANDLE if cursor is done)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_cursor_current(Handle handle, Handle *out_clause);

/**
 * @brief Check if cursor has reached the end
 * @param[in] handle Cursor handle
 * @param[out] out_is_done true if done, false otherwise
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_cursor_is_done(Handle handle, bool *out_is_done);

/**
 * @brief Reset cursor to beginning
 * @param[in] handle Cursor handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_cursor_reset(Handle handle);

/**
 * @brief Get the total number of clauses in cursor
 * @param[in] handle Cursor handle
 * @param[out] out_total Total number of clauses
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_cursor_total(Handle handle, uintptr_t *out_total);

/**
 * @brief Get the current position in cursor
 * @param[in] handle Cursor handle
 * @param[out] out_position Current position (0-indexed)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_cursor_position(Handle handle, uintptr_t *out_position);

/**
 * @brief Free a cursor
 * @param[in] handle Cursor handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_cursor_free(Handle handle);

/**
 * @brief Free a directive (usually not needed - freed with parse result)
 * @param[in] handle Directive handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_directive_free(Handle handle);

/* ============================================================================
 * Clause Query API (13 functions)
 * ========================================================================= */

/**
 * @brief Get a clause at a specific index
 * @param[in] directive_handle Directive handle
 * @param[in] index Clause index (0-based)
 * @param[out] out_clause Clause handle
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_OUT_OF_BOUNDS
 */
OmpStatus omp_clause_at(Handle directive_handle, uintptr_t index, Handle *out_clause);

/**
 * @brief Get the type of a clause
 * @param[in] handle Clause handle
 * @param[out] out_type Clause type
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_clause_type(Handle handle, ClauseType *out_type);

/**
 * @brief Get num_threads value from a num_threads clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_NUM_THREADS)
 * @param[out] out_value String handle containing the expression
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_num_threads_value(Handle handle, Handle *out_value);

/**
 * @brief Get default kind from a default clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_DEFAULT)
 * @param[out] out_kind Default kind
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_default_kind(Handle handle, DefaultKind *out_kind);

/**
 * @brief Get schedule kind from a schedule clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_SCHEDULE)
 * @param[out] out_kind Schedule kind
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_schedule_kind(Handle handle, ScheduleKind *out_kind);

/**
 * @brief Get chunk size from a schedule clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_SCHEDULE)
 * @param[out] out_chunk_size String handle (INVALID_HANDLE if no chunk size)
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_schedule_chunk_size(Handle handle, Handle *out_chunk_size);

/**
 * @brief Get reduction operator from a reduction clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_REDUCTION)
 * @param[out] out_op Reduction operator
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_reduction_operator(Handle handle, ReductionOperator *out_op);

/**
 * @brief Get custom reduction identifier from a reduction clause
 * @param[in] handle Clause handle (must be OMP_CLAUSE_REDUCTION with CUSTOM op)
 * @param[out] out_identifier String handle containing identifier
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_reduction_identifier(Handle handle, Handle *out_identifier);

/**
 * @brief Get number of items in a list clause (private, shared, etc.)
 * @param[in] handle Clause handle
 * @param[out] out_count Number of items
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_item_count(Handle handle, uintptr_t *out_count);

/**
 * @brief Get an item at index from a list clause
 * @param[in] handle Clause handle
 * @param[in] index Item index (0-based)
 * @param[out] out_item String handle containing variable name
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, OMP_OUT_OF_BOUNDS, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_item_at(Handle handle, uintptr_t index, Handle *out_item);

/**
 * @brief Check if clause is a bare clause (no arguments)
 * @param[in] handle Clause handle
 * @param[out] out_is_bare true if bare, false otherwise
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, or OMP_NULL_POINTER
 */
OmpStatus omp_clause_is_bare(Handle handle, bool *out_is_bare);

/**
 * @brief Get the bare clause name
 * @param[in] handle Clause handle (must be bare)
 * @param[out] out_name String handle containing clause name
 * @return OMP_SUCCESS, OMP_INVALID_HANDLE, OMP_NULL_POINTER, or OMP_TYPE_MISMATCH
 */
OmpStatus omp_clause_bare_name(Handle handle, Handle *out_name);

/**
 * @brief Free a clause
 * @param[in] handle Clause handle
 * @return OMP_SUCCESS or OMP_INVALID_HANDLE
 */
OmpStatus omp_clause_free(Handle handle);

/* ============================================================================
 * Utility Macros
 * ========================================================================= */

/**
 * @brief Check status and return on error
 * 
 * Usage:
 *   OMP_CHECK(omp_parse(input, OMP_LANG_C, &result));
 */
#define OMP_CHECK(call) do { \
    OmpStatus _status = (call); \
    if (_status != OMP_SUCCESS) { \
        return _status; \
    } \
} while (0)

/**
 * @brief Check status and goto cleanup label on error
 * 
 * Usage:
 *   OMP_CHECK_GOTO(omp_parse(input, lang, &result), cleanup);
 *   // ... use result ...
 *   cleanup:
 *     omp_parse_result_free(result);
 */
#define OMP_CHECK_GOTO(call, label) do { \
    OmpStatus _status = (call); \
    if (_status != OMP_SUCCESS) { \
        goto label; \
    } \
} while (0)

/**
 * @brief Check if handle is valid
 * 
 * Usage:
 *   if (OMP_IS_VALID(handle)) { ... }
 */
#define OMP_IS_VALID(handle) ((handle) != INVALID_HANDLE)

/**
 * @brief Check if handle is invalid
 */
#define OMP_IS_INVALID(handle) ((handle) == INVALID_HANDLE)

#ifdef __cplusplus
}
#endif

#endif /* ROUP_H */
