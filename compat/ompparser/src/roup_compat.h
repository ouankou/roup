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

#ifdef __cplusplus
}

const char* getPlainDirective(const OpenMPDirective* dir);
void releasePlainDirective(const OpenMPDirective* dir);
#endif

#endif /* ROUP_COMPAT_H */
