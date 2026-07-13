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

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Upstream ompparser now accepts an optional user-data pointer alongside the
 * expression callback. Redeclare the signature here so local C++ callers that
 * include roup_compat.h can continue omitting the third argument.
 */
#ifdef __cplusplus
OpenMPDirective* parseOpenMP(const char* input,
                             OpenMPExprParseCallback exprParse,
                             void* exprParseUserData = nullptr);
#else
OpenMPDirective* parseOpenMP(const char* input,
                             OpenMPExprParseCallback exprParse,
                             void* exprParseUserData);
#endif

/* Set the base language for parsing (C, C++, Fortran) */
void setLang(OpenMPBaseLang lang);

/* Thread-local diagnostic for the most recent adapter rejection. */
#ifdef __cplusplus
const char* roup_ompparser_last_error(void) noexcept;
#else
const char* roup_ompparser_last_error(void);
#endif

#ifdef __cplusplus
}
#endif

#endif /* ROUP_COMPAT_H */
