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
#include <roup_constants.h>
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Forward declarations of ROUP C API types */
typedef struct OmpDirective OmpDirective;
typedef struct OmpClause OmpClause;
typedef struct OmpClauseIterator OmpClauseIterator;
typedef struct OmpStringList OmpStringList;

/* Parsing functions */
OmpDirective* roup_parse(const char* input);
OmpDirective* roup_parse_with_language(const char* input, int32_t language);
void roup_directive_free(OmpDirective* dir);
int32_t roup_directive_kind(const OmpDirective* dir);
const char* roup_directive_name(const OmpDirective* dir);
const char* roup_directive_parameter(const OmpDirective* dir);

/* Clause iteration */
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* dir);
int roup_clause_iterator_next(OmpClauseIterator* iter, const OmpClause** clause);
void roup_clause_iterator_free(OmpClauseIterator* iter);

/* Clause data extraction */
int32_t roup_clause_kind(const OmpClause* clause);
const char* roup_clause_content(const OmpClause* clause);
OmpStringList* roup_clause_variables(const OmpClause* clause);
int32_t roup_clause_schedule_kind(const OmpClause* clause);
int32_t roup_clause_reduction_operator(const OmpClause* clause);
int32_t roup_clause_default_data_sharing(const OmpClause* clause);

/* String list operations */
int32_t roup_string_list_len(const OmpStringList* list);
const char* roup_string_list_get(const OmpStringList* list, int32_t index);
void roup_string_list_free(OmpStringList* list);

#ifdef __cplusplus
}
#endif

/* Set the base language for parsing (C, C++, Fortran) - C++ linkage for ompparser compatibility */
void setLang(OpenMPBaseLang lang);

#endif /* ROUP_COMPAT_H */
