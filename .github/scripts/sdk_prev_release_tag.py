#!/usr/bin/env python3
"""Print the latest Git tag strictly before a given semver (same tag prefix)."""
import re
import subprocess
import sys


def semver_key(s: str) -> tuple[int, ...]:
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", s)
    if not m:
        return (-1, -1, -1)
    return tuple(int(x) for x in m.groups())


def main() -> None:
    if len(sys.argv) != 3:
        print("usage: sdk_prev_release_tag.py <tagPrefix> <currentVersion>", file=sys.stderr)
        sys.exit(2)
    tag_prefix = sys.argv[1]
    current = sys.argv[2]
    out = subprocess.check_output(["git", "tag", "-l", f"{tag_prefix}*"], text=True)
    tags = [t.strip() for t in out.splitlines() if t.strip()]
    cur_key = semver_key(current)
    best: tuple[tuple[int, ...], str] | None = None
    for t in tags:
        ver = t.removeprefix(tag_prefix)
        k = semver_key(ver)
        if k < cur_key:
            if best is None or k > best[0]:
                best = (k, t)
    if best:
        print(best[1], end="")


if __name__ == "__main__":
    main()
