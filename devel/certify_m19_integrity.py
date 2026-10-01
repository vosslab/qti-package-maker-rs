#!/usr/bin/env python3
"""Certify immutable M19 ZIP outputs with the native package checker.

This reads the captured run only.  The JSON result is written outside the
immutable run directory so its ZIP and executable evidence stay untouched.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def relative_to_repository(path: Path, repository: Path) -> str:
    return path.relative_to(repository).as_posix()


def certify(repository: Path, run_directory: Path, checker: Path) -> dict[str, object]:
    packages = sorted(run_directory.rglob("*.zip"))
    if not packages:
        raise ValueError(f"no ZIP outputs found in {run_directory}")
    records: list[dict[str, object]] = []
    total_errors = 0
    total_warnings = 0
    process_failures = 0
    checker_hash_before = sha256(checker)
    for package in packages:
        package_hash_before = sha256(package)
        completed = subprocess.run(
            [str(checker), "check", str(package)],
            check=False,
            text=True,
            capture_output=True,
        )
        findings = [line for line in completed.stdout.splitlines() if line != "OK"]
        errors = [line for line in findings if line.startswith("Error\t")]
        warnings = [line for line in findings if line.startswith("Warning\t")]
        if completed.returncode != 0 and not errors:
            process_failures += 1
        total_errors += len(errors)
        total_warnings += len(warnings)
        records.append(
            {
                "path": relative_to_repository(package, repository),
                "sha256": package_hash_before,
                "exit_code": completed.returncode,
                "findings": findings,
                "stderr": completed.stderr.splitlines(),
            }
        )
    output_hashes_after = {record["path"]: sha256(repository / record["path"]) for record in records}
    changed_outputs = sorted(
        path for path, after in output_hashes_after.items() if after != next(record["sha256"] for record in records if record["path"] == path)
    )
    return {
        "schema": "qti-package-maker-rs.m19-integrity-certification.v1",
        "run_directory": relative_to_repository(run_directory, repository),
        "checker": {
            "path": relative_to_repository(checker, repository),
            "sha256": checker_hash_before,
            "sha256_after": sha256(checker),
            "command": ["qti-package-maker", "check", "<zip>"],
        },
        "package_count": len(records),
        "process_failures": process_failures,
        "error_count": total_errors,
        "warning_count": total_warnings,
        "changed_output_hashes": changed_outputs,
        "packages": records,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-directory", type=Path, required=True)
    parser.add_argument("--checker", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repository = Path.cwd().resolve()
    result = certify(repository, args.run_directory.resolve(), args.checker.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="ascii")
    print(
        f"certified {result['package_count']} ZIPs: {result['error_count']} errors, "
        f"{result['warning_count']} warnings; receipt {args.output}"
    )
    return 0 if (
        result["error_count"] == 0
        and result["process_failures"] == 0
        and not result["changed_output_hashes"]
        and result["checker"]["sha256"] == result["checker"]["sha256_after"]
    ) else 1


if __name__ == "__main__":
    sys.exit(main())
