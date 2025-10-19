# OpenACC validation with OpenACCV-V

The [`test_openacc_vv.sh`](../test_openacc_vv.sh) helper runs the official
[OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) validation suite
against ROUP's OpenACC parser. The workflow mirrors the existing OpenMP_VV
round-trip script and provides a reproducible way to triage parser regressions.

## What the script does

1. Clones the OpenACCV-V repository (or reuses an existing clone supplied via
   `OPENACC_VV_PATH`).
2. Builds the `roup_roundtrip` binary with Cargo.
3. Preprocesses every C/C++ test with `clang -E` to expand macros and includes.
4. Extracts every `#pragma acc` directive from the C/C++ sources and every
   `!$acc` directive from the Fortran test cases.
5. Pipes each directive through `roup_roundtrip` with the OpenACC dialect
   enabled.
6. Normalises both the original and round-tripped directive (`clang-format` for
   C/C++, `awk` whitespace collapsing for Fortran) and compares the results.
7. Emits a colourised summary with per-directive failure details when
   mismatches occur.

## Requirements

* `clang` and `clang-format` (any version that understands `-E` and basic
  formatting is fine). Override with `CLANG=/path` or
  `CLANG_FORMAT=/path` if necessary.
* `cargo` to build the ROUP binaries.
* `git` to fetch OpenACCV-V (skip cloning by pointing `OPENACC_VV_PATH` at an
  existing checkout).

## Usage

```bash
# Run with the default clone target (target/openacc_vv)
./test_openacc_vv.sh

# Reuse a manual clone and pick a specific clang toolchain
OPENACC_VV_PATH=$HOME/src/OpenACCV-V \
CLANG=clang-16 \
CLANG_FORMAT=clang-format-16 \
    ./test_openacc_vv.sh
```

The script exits non-zero if any directive fails to round-trip or if a parse
error occurs. Failure details include the source file, the offending directive
and either the parse error from `roup_roundtrip` or the normalised output that
failed comparison.

## Updating ROUP

When ROUP gains new OpenACC syntax support, run the script to verify that the
entire OpenACCV-V test suite still round-trips cleanly. Any mismatch should be
investigated and either fixed in the parser or documented as a known
limitation before shipping.
