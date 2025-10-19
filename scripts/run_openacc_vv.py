#!/usr/bin/env python3
"""Run OpenACCV-V directives through ROUP's OpenACC round-trip binary."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Tuple


@dataclass
class DirectiveCase:
    file_path: Path
    line: int
    language: str
    original: str


LANGUAGE_MAP = {
    ".c": "c",
    ".cc": "c",
    ".cpp": "c",
    ".cxx": "c",
    ".C": "c",
    ".F90": "fortran-free",
    ".f90": "fortran-free",
    ".F95": "fortran-free",
    ".f95": "fortran-free",
    ".F03": "fortran-free",
    ".f03": "fortran-free",
    ".F08": "fortran-free",
    ".f08": "fortran-free",
    ".F": "fortran-fixed",
    ".f": "fortran-fixed",
    ".FOR": "fortran-fixed",
    ".for": "fortran-fixed",
}

PRAGMA_REGEX = re.compile(r"^\s*#\s*pragma\s+acc\b", re.IGNORECASE)
FORTRAN_SENTINEL_REGEX = re.compile(r"^\s*[!c\*]\$\s*acc", re.IGNORECASE)
CLAUSE_COMMA_REGEX = re.compile(r"\)\s*,\s*(?=[A-Za-z])")
SPACE_BEFORE_PAREN_REGEX = re.compile(r"(?<=[A-Za-z_])\s+\(")


class TestStats:
    def __init__(self, tests_root: Path) -> None:
        self.tests_root = tests_root
        self.total_files = 0
        self.files_with_directives = 0
        self.total_directives = 0
        self.passed = 0
        self.failed = 0
        self.parse_errors = 0
        self.mismatches = 0
        self.failures: List[Tuple[DirectiveCase, str, str]] = []

    def record_pass(self) -> None:
        self.total_directives += 1
        self.passed += 1

    def record_parse_error(self, case: DirectiveCase, stderr: str) -> None:
        self.total_directives += 1
        self.failed += 1
        self.parse_errors += 1
        if len(self.failures) < 20:
            self.failures.append((case, "Parse error", stderr.strip()))

    def record_mismatch(self, case: DirectiveCase, expected: str, actual: str) -> None:
        self.total_directives += 1
        self.failed += 1
        self.mismatches += 1
        if len(self.failures) < 20:
            self.failures.append((case, expected, actual))

    def relative_path(self, case: DirectiveCase) -> str:
        try:
            return str(case.file_path.relative_to(self.tests_root))
        except ValueError:
            return str(case.file_path)


def discover_files(tests_dir: Path) -> Iterable[Path]:
    for path in sorted(tests_dir.rglob("*")):
        if not path.is_file():
            continue
        if path.suffix in LANGUAGE_MAP:
            yield path


def gather_directives(path: Path) -> List[DirectiveCase]:
    language = LANGUAGE_MAP.get(path.suffix)
    if not language:
        return []

    cases: List[DirectiveCase] = []
    lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    idx = 0
    while idx < len(lines):
        line = lines[idx]
        if language == "c":
            if not PRAGMA_REGEX.search(line):
                idx += 1
                continue
            directive_lines = [line.rstrip("\n")]
            while directive_lines[-1].rstrip().endswith("\\"):
                idx += 1
                if idx >= len(lines):
                    break
                directive_lines.append(lines[idx].rstrip("\n"))
            original = "\n".join(directive_lines)
            start_line = idx - (len(directive_lines) - 1) + 1
            cases.append(DirectiveCase(path, start_line, language, original))
            idx += 1
            continue

        # Fortran
        if not FORTRAN_SENTINEL_REGEX.search(line):
            idx += 1
            continue
        directive_lines = [line.rstrip("\n")]
        while directive_lines[-1].rstrip().endswith("&"):
            idx += 1
            if idx >= len(lines):
                break
            directive_lines.append(lines[idx].rstrip("\n"))
        original = "\n".join(directive_lines)
        start_line = idx - (len(directive_lines) - 1) + 1
        cases.append(DirectiveCase(path, start_line, language, original))
        idx += 1

    return cases


def normalize(text: str) -> str:
    text = CLAUSE_COMMA_REGEX.sub(") ", text)
    text = SPACE_BEFORE_PAREN_REGEX.sub("(", text)
    collapsed = re.sub(r"\s+", " ", text.strip())
    return collapsed.lower()


def prepare_input(text: str) -> str:
    text = CLAUSE_COMMA_REGEX.sub(") ", text)
    text = SPACE_BEFORE_PAREN_REGEX.sub("(", text)
    return text


def run_case(case: DirectiveCase, binary: Path) -> Tuple[bool, str, str]:
    prepared = prepare_input(case.original)
    proc = subprocess.run(
        [str(binary), "--lang", case.language],
        input=prepared,
        text=True,
        capture_output=True,
    )

    if proc.returncode != 0:
        return False, proc.stderr.strip(), ""

    output = proc.stdout.strip()
    original_norm = normalize(case.original)
    output_norm = normalize(output)
    success = original_norm == output_norm
    return success, output, output_norm


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tests-dir", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args(argv)

    tests_dir: Path = args.tests_dir.resolve()
    binary: Path = args.binary

    if not binary.exists():
        print(f"Error: round-trip binary not found at {binary}", file=sys.stderr)
        return 2

    stats = TestStats(tests_dir)
    for path in discover_files(tests_dir):
        stats.total_files += 1
        directives = gather_directives(path)
        if not directives:
            continue
        stats.files_with_directives += 1
        for case in directives:
            success, detail, normalized = run_case(case, binary)
            if success:
                stats.record_pass()
            else:
                if (
                    detail.startswith("Parse error")
                    or "Parse error" in detail
                    or "Unparsed trailing input" in detail
                ):
                    stats.record_parse_error(case, detail)
                else:
                    display_detail = detail if detail else normalized
                    stats.record_mismatch(
                        case,
                        normalize(case.original),
                        display_detail,
                    )

    report = {
        "files_processed": stats.total_files,
        "files_with_directives": stats.files_with_directives,
        "total_directives": stats.total_directives,
        "passed": stats.passed,
        "failed": stats.failed,
        "parse_errors": stats.parse_errors,
        "mismatches": stats.mismatches,
        "failures": [
            {
                "file": stats.relative_path(case),
                "line": case.line,
                "language": case.language,
                "reason": reason,
                "detail": detail,
                "original": case.original,
            }
            for case, reason, detail in stats.failures
        ],
    }

    if args.json_output:
        args.json_output.write_text(json.dumps(report, indent=2))

    print("=========================================")
    print("  OpenACCV-V Round-Trip Validation")
    print("=========================================")
    print("")
    print(f"Files processed:        {stats.total_files}")
    print(f"Files with directives:  {stats.files_with_directives}")
    print(f"Total directives:       {stats.total_directives}")
    print("")

    if stats.total_directives == 0:
        print("Warning: No OpenACC directives found to test")
        return 0

    pass_rate = (stats.passed / stats.total_directives) * 100.0
    print(f"Passed:                {stats.passed}")
    print(f"Failed:                {stats.failed}")
    print(f"  Parse errors:        {stats.parse_errors}")
    print(f"  Mismatches:          {stats.mismatches}")
    print("")
    print(f"Success rate:          {pass_rate:.1f}%")
    print("")

    if stats.failures:
        print("=========================================")
        print("  Failure Details (first 20)")
        print("=========================================")
        print("")
        for index, (case, reason, detail) in enumerate(stats.failures, start=1):
            print(
                f"[{index}] {stats.relative_path(case)}:{case.line} ({case.language})"
            )
            print(f"    Reason:   {reason}")
            if detail:
                print(f"    Detail:   {detail}")
            print(f"    Original: {case.original.strip()}")
            print("")

    return 0 if stats.failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
