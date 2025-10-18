/*
 * ROUP accparser compatibility API declarations
 *
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#ifndef ROUP_ACC_COMPAT_H
#define ROUP_ACC_COMPAT_H

// Forward declarations (users must include OpenACCIR.h first)
class OpenACCDirective;
enum OpenACCBaseLang;

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Parse OpenACC directive string
 *
 * @param input Pragma string (with or without "#pragma acc" prefix)
 * @param exprParse Expression parser callback (not used, pass nullptr)
 * @return Parsed directive or nullptr on error
 */
OpenACCDirective* parseOpenACC(const char* input, void* exprParse(const char* expr));

/**
 * Set the base language mode for parsing
 *
 * @param lang Language (ACC_Lang_C, ACC_Lang_Fortran, etc.)
 */
void setLang(OpenACCBaseLang lang);

#ifdef __cplusplus
}
#endif

#endif // ROUP_ACC_COMPAT_H
