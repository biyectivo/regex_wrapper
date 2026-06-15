#!/usr/bin/env python3
"""
regex - command-line wrapper around Python's `re` module.

Usage:
    regex search   <string> <pattern> [<flags>]
    regex findall  <string> <pattern> [<flags>]
    regex split    <string> <pattern> [<flags>]
    regex sub      <string> <pattern> <replacement> [<flags>]

Exit codes:
    0  success
    1  invalid regex or usage error
"""

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
    #print(f"[{timestamp}] args: {args_str} | result: {result}\n")

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
            print(f"Unknown flag: {token}", file=sys.stderr)
            sys.exit(1)
        result |= flag_val
    return result


def main() -> None:
    args = sys.argv[1:]

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

    flags = parse_flags(flags_str)

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
