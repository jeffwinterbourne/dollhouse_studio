#!/usr/bin/env python3
"""Public repository hygiene and sanitisation checker for Dollhouse Studio."""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

FORBIDDEN_PATH_PARTS = {
    ".gemini",
    ".claude",
    ".cursor",
    ".windsurf",
    ".vscode",
    ".idea",
    ".autosave",
}

FORBIDDEN_FILENAMES = {
    "CLAUDE.md",
    ".env",
    "Thumbs.db",
    ".DS_Store",
}

FORBIDDEN_EXTENSIONS = {".pem", ".key", ".stl"}

# Patterns constructed dynamically so this script does not embed literal personal/secret strings.
CONTENT_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("Windows local drive path", re.compile(r"(?i)(?:^|[^A-Za-z0-9_])[c-z]:\\\\")),
    ("Windows local drive slash path", re.compile(r"(?i)(?:^|[^A-Za-z0-9_])[c-z]:/(?!/)")),
    ("Unix home directory path", re.compile(r"(?:/Users/|/home/)[A-Za-z0-9._-]+")),
    ("GitHub classic PAT", re.compile(r"ghp_[A-Za-z0-9]{20,}")),
    ("GitHub fine-grained PAT", re.compile(r"github_pat_[A-Za-z0-9_]{20,}")),
    ("AWS access key", re.compile(r"AKIA[0-9A-Z]{16}")),
    ("Private key block", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    (
        "Agentic workspace marker",
        re.compile(
            "|".join(
                [
                    r"\." + "gemini",
                    r"\." + "claude",
                    r"\." + "cursor",
                    "anti" + "gravity",
                    "claude" + r"\.md",
                ]
            ),
            re.IGNORECASE,
        ),
    ),
]

TEXT_EXTENSIONS = {
    ".rs",
    ".toml",
    ".md",
    ".txt",
    ".yml",
    ".yaml",
    ".json",
    ".py",
    ".ps1",
    ".sh",
    ".nsi",
    ".plist",
    ".svg",
    ".gitignore",
}


def tracked_and_untracked_files(repo_root: pathlib.Path) -> list[pathlib.Path]:
    res = subprocess.run(
        ["git", "-C", str(repo_root), "ls-files", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        check=True,
    )
    out: list[pathlib.Path] = []
    for line in res.stdout.splitlines():
        line = line.strip()
        if line:
            out.append(repo_root / line)
    return out


def is_text_candidate(path: pathlib.Path) -> bool:
    if path.name in {"LICENSE", ".gitignore", "Dockerfile"}:
        return True
    return path.suffix.lower() in TEXT_EXTENSIONS


def main() -> int:
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    files = tracked_and_untracked_files(repo_root)
    violations: list[str] = []

    for path in files:
        if not path.is_file():
            continue
        rel = path.relative_to(repo_root).as_posix()
        parts = set(path.relative_to(repo_root).parts)
        if parts & FORBIDDEN_PATH_PARTS:
            violations.append(f"[forbidden-dir] {rel}")
        if path.name in FORBIDDEN_FILENAMES:
            violations.append(f"[forbidden-file] {rel}")
        if path.suffix.lower() in FORBIDDEN_EXTENSIONS:
            violations.append(f"[forbidden-ext] {rel}")

        if rel == "scripts/hygiene_check.py" or not is_text_candidate(path):
            continue

        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        for lineno, line in enumerate(text.splitlines(), start=1):
            for label, pat in CONTENT_PATTERNS:
                if rel == ".gitignore" and label == "Agentic workspace marker":
                    continue
                if pat.search(line):
                    violations.append(f"[{label}] {rel}:{lineno}: {line.strip()[:120]}")

    if violations:
        print("HYGIENE CHECK FAILED:", file=sys.stderr)
        for v in violations:
            print(f"  - {v}", file=sys.stderr)
        return 1

    print(f"Hygiene check passed ({len(files)} file(s) scanned, 0 violations).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
