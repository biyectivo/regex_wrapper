#!/usr/bin/env python3
"""
Build regex.py into a single-file executable using Nuitka.

Usage:
    python build.py [--os windows|linux|macos]

Requires: pip install nuitka
"""

import argparse
import platform
import subprocess
import sys


def main() -> None:
    parser = argparse.ArgumentParser(description="Build regex into a single-file executable.")
    parser.add_argument(
        "--os",
        choices=["windows", "linux", "macos"],
        default=None,
        help="Target OS (default: auto-detect current platform).",
    )
    args = parser.parse_args()

    # Detect current platform if not specified
    if args.os is None:
        plat = platform.system().lower()
        os_map = {"windows": "windows", "linux": "linux", "darwin": "macos"}
        args.os = os_map.get(plat, plat)

    # Warn if cross-compiling (Nuitka doesn't support it)
    current = platform.system().lower()
    current_norm = {"darwin": "macos"}.get(current, current)
    if current_norm != args.os:
        print(
            f"WARNING: You are on {current_norm} but targeting {args.os}. "
            f"Nuitka cannot cross-compile. Run this script on the target OS.",
            file=sys.stderr,
        )
        sys.exit(1)

    exe_name = "regex.exe" if args.os == "windows" else "regex"

    cmd = [
        sys.executable, "-m", "nuitka",
        "--onefile",
        "--lto=yes",
        "--output-filename=" + exe_name,
        "--output-dir=dist",
        "--remove-output",
        "regex.py",
    ]

    print(f"Building for {args.os}...")
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd)
    if result.returncode != 0:
        print("Build failed.", file=sys.stderr)
        sys.exit(1)

    print(f"\nSuccess! Executable: dist/{exe_name}")


if __name__ == "__main__":
    main()
