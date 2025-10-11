/**
 * @file string_builder.c
 * @brief Demonstrates the string building API
 * 
 * This example shows:
 * - Creating new strings
 * - Building strings incrementally
 * - String operations (length, capacity, clear)
 * - Converting between C strings and handles
 * - Proper memory management
 * 
 * Build:
 *   gcc -o string_builder string_builder.c -L../../target/debug -lroup -lpthread -ldl -lm
 * 
 * Run:
 *   LD_LIBRARY_PATH=../../target/debug ./string_builder
 */

#include "../../include/roup.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Print string info
 */
void print_string_info(const char *label, Handle str) {
    uintptr_t len, capacity;
    bool is_empty;
    
    omp_str_len(str, &len);
    omp_str_capacity(str, &capacity);
    omp_str_is_empty(str, &is_empty);
    
    printf("%s:\n", label);
    printf("  Length: %zu\n", len);
    printf("  Capacity: %zu\n", capacity);
    printf("  Empty: %s\n", is_empty ? "yes" : "no");
    
    if (!is_empty && len > 0) {
        char *buffer = malloc(len + 1);
        if (buffer) {
            uintptr_t written;
            if (omp_str_copy_to_buffer(str, buffer, len + 1, &written) == OMP_SUCCESS) {
                printf("  Content: \"%s\"\n", buffer);
            }
            free(buffer);
        }
    }
}

int main() {
    printf("=== String Builder Example ===\n\n");
    
    // Example 1: Create empty string
    printf("Example 1: Create and build a string\n\n");
    
    Handle str1;
    if (omp_str_new(&str1) != OMP_SUCCESS) {
        printf("Error: Failed to create string\n");
        return 1;
    }
    
    print_string_info("Empty string", str1);
    
    // Add content
    printf("\nAppending \"Hello\"...\n");
    omp_str_push_cstr(str1, "Hello");
    print_string_info("After first append", str1);
    
    printf("\nAppending \" World\"...\n");
    omp_str_push_cstr(str1, " World");
    print_string_info("After second append", str1);
    
    printf("\nAppending \"!\"...\n");
    omp_str_push_cstr(str1, "!");
    print_string_info("After third append", str1);
    
    // Clear
    printf("\nClearing string...\n");
    omp_str_clear(str1);
    print_string_info("After clear", str1);
    
    omp_str_free(str1);
    
    // Example 2: Create from C string
    printf("\n----------------------------------------\n");
    printf("Example 2: Create from C string\n\n");
    
    Handle str2;
    const char *source = "OpenMP Directive";
    if (omp_str_from_cstr(source, &str2) == OMP_SUCCESS) {
        printf("Created from: \"%s\"\n\n", source);
        print_string_info("Created string", str2);
        omp_str_free(str2);
    }
    
    // Example 3: Build complex string
    printf("\n----------------------------------------\n");
    printf("Example 3: Build a complex string incrementally\n\n");
    
    Handle str3;
    omp_str_new(&str3);
    
    const char *parts[] = {
        "#pragma omp ",
        "parallel ",
        "for ",
        "schedule(dynamic, 10) ",
        "reduction(+: sum)"
    };
    
    printf("Building string from parts:\n");
    for (int i = 0; i < 5; i++) {
        printf("  Adding: \"%s\"\n", parts[i]);
        omp_str_push_cstr(str3, parts[i]);
    }
    
    printf("\n");
    print_string_info("Final result", str3);
    omp_str_free(str3);
    
    // Example 4: Byte-level manipulation
    printf("\n----------------------------------------\n");
    printf("Example 4: Byte-level string building\n\n");
    
    Handle str4;
    omp_str_new(&str4);
    
    // Add bytes directly
    const uint8_t bytes1[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F}; // "Hello"
    const uint8_t bytes2[] = {0x20, 0x52, 0x75, 0x73, 0x74}; // " Rust"
    
    printf("Adding bytes: [0x48, 0x65, 0x6C, 0x6C, 0x6F] (\"Hello\")\n");
    omp_str_push_bytes(str4, bytes1, 5);
    
    printf("Adding bytes: [0x20, 0x52, 0x75, 0x73, 0x74] (\" Rust\")\n\n");
    omp_str_push_bytes(str4, bytes2, 5);
    
    print_string_info("Byte-built string", str4);
    omp_str_free(str4);
    
    // Example 5: Multiple strings
    printf("\n----------------------------------------\n");
    printf("Example 5: Working with multiple strings\n\n");
    
    Handle strings[3];
    const char *contents[] = {
        "First string",
        "Second string", 
        "Third string"
    };
    
    // Create all strings
    for (int i = 0; i < 3; i++) {
        omp_str_from_cstr(contents[i], &strings[i]);
    }
    
    // Print all
    for (int i = 0; i < 3; i++) {
        char label[32];
        snprintf(label, sizeof(label), "String %d", i + 1);
        print_string_info(label, strings[i]);
        printf("\n");
    }
    
    // Free all
    for (int i = 0; i < 3; i++) {
        omp_str_free(strings[i]);
    }
    
    printf("=== All examples completed successfully ===\n");
    return 0;
}
