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

/* Set the base language for parsing (C, C++, Fortran) */
void setLang(OpenMPBaseLang lang);

/* Retrieve the normalized plain directive string */
const char* roup_get_plain_directive_string(const OpenMPDirective* dir);

#ifdef __cplusplus
}
#endif

#endif /* ROUP_COMPAT_H */
