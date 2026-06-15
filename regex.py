#!/usr/bin/env python3
"""
regex - command-line wrapper around Python's `re` module.

Usage:
    regex search   <string> <pattern> [<flags>]
    regex findall  <string> <pattern> [<flags>]
    regex split    <string> <pattern> [<flags>]
    regex sub      <string> <pattern> <replacement> [<flags>]

    regex --json   (read JSON array from stdin, write JSON array to stdout)

Exit codes:
    0  success
    1  invalid regex or usage error
"""

import json
import os
import re
import sys
from datetime import datetime


DEBUG = os.environ.get("REGEX_WRAPPER_DEBUG") == "1"
LOG_FILE = os.path.join(os.path.dirname(os.path.abspath(sys.argv[0])), "regex_wrapper.log")


def debug_log(result: str) -> None:
    """Append a timestamped entry with args and result to the log file."""
    if not DEBUG:
        return
    timestamp = datetime.now().isoformat(sep=" ", timespec="milliseconds")
    args_str = " ".join(repr(a) for a in sys.argv[1:])
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(f"[{timestamp}] args: {args_str} | result: {result}\n")


def debug_log_batch(entry: dict, result) -> None:
    """Append a timestamped entry for a batch item to the log file."""
    if not DEBUG:
        return
    timestamp = datetime.now().isoformat(sep=" ", timespec="milliseconds")
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(f"[{timestamp}] batch: {json.dumps(entry)} | result: {json.dumps(result)}\n")


def parse_flags(flag_str: str) -> re.RegexFlag:
    """
    Accept a flags string like "re.IGNORECASE|re.MULTILINE" or "IGNORECASE|MULTILINE"
    or shorthand letters "im" (i=IGNORECASE, m=MULTILINE, etc.).
    Returns the combined RegexFlag value.
    """
    if not flag_str:
        return re.RegexFlag(0)

    # Shorthand single-letter map (matches inline-flag letters)
    shorthand = {
        "i": re.IGNORECASE,
        "m": re.MULTILINE,
        "s": re.DOTALL,
        "x": re.VERBOSE,
        "a": re.ASCII,
        "u": re.UNICODE,
    }

    # If every character is a known shorthand letter, treat as shorthand
    if all(ch in shorthand for ch in flag_str):
        result = re.RegexFlag(0)
        for ch in flag_str:
            result |= shorthand[ch]
        return result

    # Otherwise parse "re.IGNORECASE|re.DOTALL" or "IGNORECASE|DOTALL"
    result = re.RegexFlag(0)
    for token in flag_str.split("|"):
        token = token.strip().removeprefix("re.")
        flag_val = getattr(re, token, None)
        if flag_val is None or not isinstance(flag_val, re.RegexFlag):
            raise ValueError(f"Unknown flag: {token}")
        result |= flag_val
    return result


def execute_one(entry: dict):
    """Execute a single regex operation from a dict. Returns the result value."""
    instruction = entry.get("instruction", "").lower()
    string = entry.get("string", "")
    pattern = entry.get("pattern", "")
    flags_str = entry.get("flags", "")
    replacement = entry.get("replacement", "")

    valid = ("search", "findall", "split", "sub")
    if instruction not in valid:
        return {"error": f"Unknown instruction: {instruction!r}"}

    try:
        flags = parse_flags(flags_str)
    except ValueError as e:
        return {"error": str(e)}

    try:
        compiled = re.compile(pattern, flags)
    except re.error:
        return False

    if instruction == "search":
        m = compiled.search(string)
        return m.start() if m else -1

    elif instruction == "findall":
        return compiled.findall(string)

    elif instruction == "split":
        return compiled.split(string)

    elif instruction == "sub":
        return compiled.sub(replacement, string)


def run_batch() -> None:
    """Read a JSON array from stdin, execute each entry, write JSON array to stdout."""
    raw = sys.stdin.read()
    try:
        commands = json.loads(raw)
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"Invalid JSON: {e}"}), file=sys.stderr)
        sys.exit(1)

    if not isinstance(commands, list):
        print(json.dumps({"error": "Expected a JSON array"}), file=sys.stderr)
        sys.exit(1)

    results = []
    for entry in commands:
        result = execute_one(entry)
        debug_log_batch(entry, result)
        results.append(result)

    print(json.dumps(results))


def main() -> None:
    args = sys.argv[1:]

    # JSON batch mode
    if args and args[0] == "--json":
        run_batch()
        return

    if len(args) < 3:
        print(__doc__.strip(), file=sys.stderr)
        sys.exit(1)

    instruction = args[0].lower()
    valid = ("search", "findall", "split", "sub")
    if instruction not in valid:
        print(f"Unknown instruction: {instruction!r}  (expected one of {valid})", file=sys.stderr)
        sys.exit(1)

    string = args[1]
    pattern = args[2]

    # For "sub" we need an extra <replacement> argument before optional flags
    if instruction == "sub":
        if len(args) < 4:
            print("'sub' requires a <replacement> argument after <pattern>.", file=sys.stderr)
            sys.exit(1)
        replacement = args[3]
        flags_str = args[4] if len(args) > 4 else ""
    else:
        replacement = ""
        flags_str = args[3] if len(args) > 3 else ""

    try:
        flags = parse_flags(flags_str)
    except ValueError as e:
        print(str(e), file=sys.stderr)
        sys.exit(1)

    # Compile the pattern — invalid regex returns "False"
    try:
        compiled = re.compile(pattern, flags)
    except re.error:
        print("False")
        debug_log("False")
        sys.exit(1)

    # Dispatch
    if instruction == "search":
        m = compiled.search(string)
        if m is None:
            print(-1)
            debug_log("-1")
        else:
            print(m.start())
            debug_log(str(m.start()))

    elif instruction == "findall":
        result = compiled.findall(string)
        print(result)
        debug_log(repr(result))

    elif instruction == "split":
        result = compiled.split(string)
        print(result)
        debug_log(repr(result))

    elif instruction == "sub":
        result = compiled.sub(replacement, string)
        print(result)
        debug_log(repr(result))


if __name__ == "__main__":
    main()
