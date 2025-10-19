#!/usr/bin/env python3
"""OpenACCV-V validation runner.

This script scans the OpenACCV-V test suite, extracts every OpenACC directive,
round-trips it through the `roup_roundtrip` binary, and reports pass/fail
statistics.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional

VALID_EXTENSIONS = {
    ".c",
    ".cc",
    ".cpp",
    ".cxx",
    ".h",
    ".hh",
    ".hpp",
    ".hxx",
    ".f",
    ".f90",
    ".f95",
    ".f03",
    ".f08",
    ".F",
    ".F90",
    ".F95",
    ".F03",
    ".F08",
}


@dataclass
class Directive:
    path: Path
    start_line: int
    lines: List[str]
    language: str  # "c" or "fortran"
    prefix: str

    def parser_input(self) -> str:
        cleaned: List[str] = []
        for idx, raw_line in enumerate(self.lines):
            if self.language == "c":
                line = _strip_c_comment(raw_line)
            else:
                line = _strip_fortran_comment(raw_line, idx == 0, len(self.prefix))
            cleaned.append(line.rstrip())
        result = "\n".join(part for part in cleaned if part.strip())
        return _normalize_directive_text(result)

    def canonicalize(self, text: str) -> str:
        c_style = self._to_c_style(text)
        if not c_style:
            return ""
        return " ".join(c_style.split())

    def _to_c_style(self, text: str) -> str:
        parts: List[str] = []
        lines = [segment for segment in text.strip().splitlines() if segment.strip()]
        if self.language == "c":
            for line in lines:
                stripped = line.strip()
                if stripped.endswith("\\"):
                    stripped = stripped[:-1].rstrip()
                parts.append(stripped)
            joined = " ".join(parts)
            return _normalize_directive_text(joined)

        # Fortran: strip sentinel/continuation markers, then convert to #pragma form
        for idx, line in enumerate(lines):
            stripped = line.strip()
            if idx == 0:
                body = stripped[len(self.prefix) :].lstrip()
            else:
                body = stripped.lstrip()
            if body.startswith("&"):
                body = body[1:].lstrip()
            if body.endswith("&"):
                body = body[:-1].rstrip()
            if body:
                parts.append(body)
        joined = " ".join(parts).strip()
        if joined:
            return _normalize_directive_text(f"#pragma acc {joined}")
        return "#pragma acc"


def _strip_c_comment(line: str) -> str:
    trimmed = line.rstrip()
    if "//" in trimmed:
        idx = trimmed.find("//")
        return trimmed[:idx].rstrip()
    return trimmed


def _strip_fortran_comment(line: str, is_first: bool, prefix_len: int) -> str:
    trimmed = line.rstrip()
    start = prefix_len if is_first else 0
    for idx, ch in enumerate(trimmed):
        if idx < start:
            continue
        if ch == "!":
            return trimmed[:idx].rstrip()
    return trimmed


def _detect_directives(path: Path) -> Iterable[Directive]:
    try:
        content = path.read_text()
    except UnicodeDecodeError:
        return []

    directives: List[Directive] = []
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        original_line = lines[i]
        stripped = original_line.lstrip()
        lower = stripped.lower()
        directive_info: Optional[tuple[str, str]] = None
        if lower.startswith("#pragma acc"):
            # Preserve prefix with original casing/spacing
            prefix_end = len(stripped.split(None, 2)[0])
            prefix = stripped[:prefix_end]
            directive_info = ("c", prefix)
        elif lower.startswith("!$acc") or lower.startswith("c$acc") or lower.startswith("*$acc"):
            prefix = stripped[:5]
            directive_info = ("fortran", prefix)

        if directive_info is None:
            i += 1
            continue

        language, prefix = directive_info
        start_line = i + 1
        collected = [original_line]

        if language == "c":
            while collected[-1].rstrip().endswith("\\") and i + 1 < len(lines):
                i += 1
                collected.append(lines[i])
        else:
            while collected[-1].rstrip().endswith("&") and i + 1 < len(lines):
                i += 1
                collected.append(lines[i])
        directives.append(Directive(path, start_line, collected, language, prefix))
        i += 1
    return directives


def run_roundtrip(directive: Directive, roundtrip_bin: Path) -> subprocess.CompletedProcess[str]:
    parser_input = directive.parser_input()
    return subprocess.run(
        [roundtrip_bin],
        input=parser_input,
        text=True,
        capture_output=True,
    )


def _normalize_directive_text(text: str) -> str:
    text = text.replace("\n", " ")
    text = text.replace("),", ") ")
    text = re.sub(r"(?<=\w)\s+\(", "(", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description="Run OpenACCV-V validation")
    parser.add_argument("--tests-dir", required=True, help="Path to OpenACCV-V Tests directory")
    parser.add_argument("--roundtrip-bin", required=True, help="Path to roup_roundtrip binary")
    parser.add_argument(
        "--max-failures",
        type=int,
        default=10,
        help="Maximum number of failures to display",
    )
    args = parser.parse_args()

    tests_dir = Path(args.tests_dir)
    if not tests_dir.is_dir():
        print(f"Error: Tests directory not found: {tests_dir}")
        return 1

    roundtrip_bin = Path(args.roundtrip_bin)
    if not roundtrip_bin.exists():
        print(f"Error: roundtrip binary not found: {roundtrip_bin}")
        return 1

    all_files = sorted(
        path
        for path in tests_dir.rglob("*")
        if path.is_file() and path.suffix in VALID_EXTENSIONS
    )

    total_files = len(all_files)
    files_with_directives = 0
    total_directives = 0
    c_directives = 0
    fortran_directives = 0
    passed = 0
    failed = 0
    parse_errors = 0
    failures = []

    for file_path in all_files:
        directives = list(_detect_directives(file_path))
        if not directives:
            continue
        files_with_directives += 1
        for directive in directives:
            parser_input = directive.parser_input()
            if not parser_input:
                continue
            total_directives += 1
            if directive.language == "c":
                c_directives += 1
            else:
                fortran_directives += 1

            result = run_roundtrip(directive, roundtrip_bin)
            if result.returncode != 0:
                failed += 1
                parse_errors += 1
                failures.append(
                    {
                        "path": directive.path,
                        "line": directive.start_line,
                        "reason": result.stderr.strip() or "Parse error",
                        "original": parser_input,
                        "roundtrip": "",
                    }
                )
                continue

            output = result.stdout.strip()
            original_norm = directive.canonicalize(parser_input)
            roundtrip_norm = directive.canonicalize(output)
            if original_norm == roundtrip_norm:
                passed += 1
            else:
                failed += 1
                failures.append(
                    {
                        "path": directive.path,
                        "line": directive.start_line,
                        "reason": "Normalized mismatch",
                        "original": original_norm,
                        "roundtrip": roundtrip_norm,
                    }
                )

    success_rate = (passed * 100.0 / total_directives) if total_directives else 0.0

    print("=========================================")
    print("  OpenACCV-V Round-Trip Validation")
    print("=========================================")
    print()
    print(f"Files processed:        {total_files}")
    print(f"Files with directives:  {files_with_directives}")
    print(f"Total directives:       {total_directives}")
    print(f"  C/C++ directives:     {c_directives}")
    print(f"  Fortran directives:   {fortran_directives}")
    print()
    print(f"Passed:                 {passed}")
    print(f"Failed:                 {failed}")
    print(f"  Parse errors:         {parse_errors}")
    print(f"  Mismatches:           {failed - parse_errors}")
    print()
    print(f"Success rate:           {success_rate:.1f}%")
    print()

    if failures:
        print("=========================================")
        print(
            f"  Failure Details (showing first {min(len(failures), args.max_failures)})"
        )
        print("=========================================")
        print()
        for failure in failures[: args.max_failures]:
            print(f"{failure['path']}:{failure['line']}")
            print(f"    Reason:    {failure['reason']}")
            if failure["original"]:
                print(f"    Expected:  {failure['original']}")
            if failure["roundtrip"]:
                print(f"    Roundtrip: {failure['roundtrip']}")
            print()
        if len(failures) > args.max_failures:
            remaining = len(failures) - args.max_failures
            print(f"... and {remaining} more failures")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
