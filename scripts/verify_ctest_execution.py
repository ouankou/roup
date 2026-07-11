#!/usr/bin/env python3
"""Require a complete, passing CTest execution receipt."""

from __future__ import annotations

import argparse
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import NoReturn


def fail(message: str) -> NoReturn:
    raise SystemExit(f"error: {message}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("expected_count", type=int)
    parser.add_argument("suite_name")
    args = parser.parse_args()

    if args.expected_count <= 0:
        fail("expected test count must be positive")
    if not args.report.is_file():
        fail(f"CTest did not produce execution receipt {args.report}")

    try:
        root = ET.parse(args.report).getroot()
    except ET.ParseError as error:
        fail(f"malformed CTest execution receipt {args.report}: {error}")

    if root.tag != "testsuite":
        fail(f"unexpected CTest receipt root {root.tag!r}")

    test_cases = root.findall("testcase")
    reported_count = root.get("tests")
    if reported_count != str(args.expected_count):
        fail(
            f"{args.suite_name} receipt reports {reported_count!r} tests; "
            f"expected {args.expected_count}"
        )
    if len(test_cases) != args.expected_count:
        fail(
            f"{args.suite_name} receipt contains {len(test_cases)} executions; "
            f"expected {args.expected_count}"
        )

    names = [test_case.get("name") for test_case in test_cases]
    if any(name is None or not name for name in names):
        fail(f"{args.suite_name} receipt contains an unnamed test")
    if len(set(names)) != len(names):
        fail(f"{args.suite_name} receipt contains duplicate test executions")

    for result_kind in ("failure", "error", "skipped"):
        offenders = [
            test_case.get("name", "<unnamed>")
            for test_case in test_cases
            if test_case.find(result_kind) is not None
        ]
        if offenders:
            fail(
                f"{args.suite_name} receipt contains {result_kind} results: "
                + ", ".join(offenders[:10])
            )

    missing_times = [
        test_case.get("name", "<unnamed>")
        for test_case in test_cases
        if test_case.get("time") is None
    ]
    if missing_times:
        fail(
            f"{args.suite_name} receipt lacks execution times for: "
            + ", ".join(missing_times[:10])
        )

    print(
        f"verified complete {args.suite_name} execution: "
        f"{len(test_cases)} unique passing tests recorded by CTest"
    )


if __name__ == "__main__":
    main()
