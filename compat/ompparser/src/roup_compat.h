/*
 * roup_compat.h - Additional declarations for ROUP ompparser compatibility
 *
 * This provides function declarations that extend ompparser for ROUP use
 *
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#ifndef ROUP_COMPAT_H
#define ROUP_COMPAT_H

#include <OpenMPIR.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Set the base language for parsing (C, C++, Fortran) */
void setLang(OpenMPBaseLang lang);

/* Convert directives between languages (C/C++ <-> Fortran). */
char* roup_convert_language(const char* input, int32_t from_lang, int32_t to_lang);

/* Free strings returned by roup_convert_language(). */
void roup_string_free(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* ROUP_COMPAT_H */
