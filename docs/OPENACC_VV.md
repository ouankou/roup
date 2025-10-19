# OpenACC validation with OpenACCV-V

The [`test_openacc_vv.sh`](../test_openacc_vv.sh) helper runs the official
[OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) validation suite
against ROUP's OpenACC parser. The workflow mirrors the existing OpenMP_VV
round-trip script and provides a reproducible way to triage parser regressions.

## What the script does

1. Clones the OpenACCV-V repository (or reuses an existing clone supplied via
   `OPENACC_VV_PATH`).
2. Builds the `roup_roundtrip` binary with Cargo.
3. Finds all C/C++/Fortran source files under `Tests/`.
4. Extracts every OpenACC directive directly from source files (no preprocessing):
   - C/C++: `#pragma acc` directives
   - Fortran: `!$acc`, `c$acc`, `*$acc` directives
5. Pipes each directive through `roup_roundtrip --acc` with the OpenACC dialect
   enabled.
6. Normalizes both the original and round-tripped directive to handle OpenACC's
   optional formatting (commas between clauses, space before parentheses) and
   compares the results.
7. Emits a summary with per-directive failure details when
   mismatches occur.

## Requirements

* `cargo` to build the ROUP binaries.
* `git` to fetch OpenACCV-V (skip cloning by pointing `OPENACC_VV_PATH` at an
  existing checkout).

## Usage

```bash
# Run with the default clone target (target/openacc_vv)
./test_openacc_vv.sh

# Reuse a manual clone
OPENACC_VV_PATH=$HOME/src/OpenACCV-V ./test_openacc_vv.sh
```

The script exits non-zero if any directive fails to round-trip or if a parse
error occurs. Failure details include the source file, the offending directive
and either the parse error from `roup_roundtrip` or the normalized output that
failed comparison.

## Updating ROUP

When ROUP gains new OpenACC syntax support, run the script to verify that the
entire OpenACCV-V test suite still round-trips cleanly. Any mismatch should be
investigated and either fixed in the parser or documented as a known
limitation before shipping.
