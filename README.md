# regex_wrapper

A command-line wrapper around Rust's [regex](https://docs.rs/regex/) engine. Provides regex operations (`search`, `findall`, `split`, `sub`) callable directly from the shell, with support for both single-command CLI usage and JSON batch mode for processing multiple operations in one invocation.

Built in Rust for near-instant startup (~2-5ms) and small binary size (~1-3 MB).

## Features

- Direct CLI access to a high-performance regex engine
- JSON batch mode for multiple operations without repeated process startup
- Compiles to a single statically-linked native executable (Windows, Linux, macOS)
- Optional debug logging (timestamped args + results)
- Invalid regex patterns return `False` instead of raising exceptions
- Cross-platform: builds natively on Windows, Linux, and macOS

## Requirements

- [Rust](https://rustup.rs/) (1.75+ recommended)

Install Rust:
```bash
# Windows (PowerShell)
winget install Rustlang.Rustup

# Linux / macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

```bash
cargo build --release
```

The executable is output to `target/release/regex.exe` (Windows) or `target/release/regex` (Linux/macOS).

Copy it to a directory on your `PATH` to use it globally.

### Cross-compilation

To build for a different target (e.g., Linux from Windows), add the target and build:

```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

For CI, use a GitHub Actions matrix with `windows-latest`, `ubuntu-latest`, `macos-latest`.

## Usage

### CLI mode

```
regex <instruction> <string> <pattern> [<flags>]
regex sub <string> <pattern> <replacement> [<flags>]

# Use --file to read the target string from a file instead:
regex <instruction> --file <path> <pattern> [<flags>]
regex sub --file <path> <pattern> <replacement> [<flags>]
```

**Instructions:**

| Instruction | Description             | Output                                 |
| -------------| -------------------------| ----------------------------------------|
| `search`    | Find first match        | Start index (>= 0) or `-1` if no match |
| `findall`   | Find all matches        | JSON array of matched strings          |
| `split`     | Split string by pattern | JSON array of parts                    |
| `sub`       | Substitute matches      | Resulting string                       |

**Flags** (optional, 4th or 5th argument):

- Shorthand letters: `i`, `m`, `s`, `x`, `a`, `u` (e.g. `"im"`)
- Named: `IGNORECASE|MULTILINE`
- Prefixed: `re.IGNORECASE|re.MULTILINE`

| Letter | Named | Effect |
|---|---|---|
| `i` | `IGNORECASE` | Case-insensitive matching |
| `m` | `MULTILINE` | `^` and `$` match line boundaries |
| `s` | `DOTALL` | `.` matches newline |
| `x` | `VERBOSE` | Ignore whitespace and `#` comments in pattern |
| `a` | `ASCII` | Disable Unicode matching (ASCII only) |
| `u` | `UNICODE` | Enable Unicode matching (default) |

**Examples:**

```bash
regex search "hello world" "\w+"
# Output: 0

regex search "hello world" "xyz"
# Output: -1

regex findall "one 1 two 2 three 3" "\d+"
# Output: ["1", "2", "3"]

regex split "one,two,,three" ","
# Output: ["one", "two", "", "three"]

regex sub "hello world" "world" "Rust"
# Output: hello Rust

regex search "Hello World" "hello" i
# Output: 0

regex search "test" "[invalid"
# Output: False

# Read target string from a file (avoids shell escaping issues with HTML, etc.)
regex sub --file page.html '("[^"]*")| ' '${1}¬'
```

### JSON batch mode

Pass a JSON array with the `--json` flag. Each element is an object describing one operation. Returns a JSON array of results.

```
regex --json                             # read from stdin, write to stdout
regex --json <input_file>                # read from file, write to stdout
regex --json <input_file> <output_file>  # read from file, write to file
```

**Examples:**

```bash
# Pipe from stdin (original behavior)
echo '[{"instruction":"search","string":"hello world","pattern":"\\w+"},{"instruction":"findall","string":"one 1 two 2","pattern":"\\d+"}]' | regex --json

# Read from a file
regex --json commands.json

# Read from a file, write results to a file
regex --json commands.json results.json
```

Output:
```json
[0,["1","2"]]
```

**Entry format:**

```json
{
  "instruction": "search|findall|split|sub",
  "string": "target string",
  "string_file": "path/to/file.txt (alternative to string)",
  "pattern": "regex pattern",
  "flags": "optional flags string",
  "replacement": "required for sub"
}
```

Use `string_file` instead of `string` to read the target string from a file. This avoids shell and JSON escaping issues when processing content like HTML:

```json
[
  {
    "instruction": "sub",
    "string_file": "page.html",
    "pattern": "(\"[^\"]*\")| ",
    "replacement": "${1}¬"
  }
]
```

If both `string` and `string_file` are provided, `string_file` takes priority. If the file cannot be read, the entry returns `{"error": "..."}`.

**Chaining with `$PREV$`:**

Use `$PREV$` in the `string` field to reference the result of the previous entry. This lets you chain multiple regex operations together:

```json
[
  {
    "instruction": "sub",
    "string_file": "page.html",
    "pattern": "(\"[^\"]*\")| ",
    "replacement": "${1}¬"
  },
  {
    "instruction": "sub",
    "string": "$PREV$",
    "pattern": "'([^'¬]*)¬([^']*)'",
    "replacement": "'$1 $2'"
  },
  {
    "instruction": "sub",
    "string": "$PREV$",
    "pattern": "'([^'¬]*)¬([^']*)'",
    "replacement": "'$1 $2'"
  }
]
```

`$PREV$` can also appear as part of a larger string (e.g. `"prefix$PREV$suffix"`). For string results (from `sub`), `$PREV$` expands to the raw string. For non-string results (from `findall`, `split`, `search`), it expands to their JSON representation.

Using `$PREV$` on the first entry (no previous result) returns `{"error": "..."}`.

**Error handling in batch mode:**

- Invalid regex returns `false` for that entry
- Unknown instruction returns `{"error": "..."}`
- Unreadable `string_file` returns `{"error": "..."}`
- `$PREV$` on first entry returns `{"error": "..."}`
- Invalid JSON on stdin/in file prints an error to stderr and exits with code 1
- Non-existent input file prints an error to stderr and exits with code 1

## Debug logging

Set the environment variable `REGEX_WRAPPER_DEBUG=1` to enable logging. A file named `regex_wrapper.log` is created/appended in the same directory as the executable.

```bash
# PowerShell
$env:REGEX_WRAPPER_DEBUG="1"
regex search "hello" "\w+"

# bash
REGEX_WRAPPER_DEBUG=1 regex search "hello" "\w+"
```

Log format:
```
[2026-06-14 21:51:12.339] args: "search" "hello" "\\w+" | result: 0
[2026-06-14 21:51:12.500] batch: {"instruction":"search",...} | result: 0
```

## Gotchas, tips, and caveats

### Shell quoting (the biggest pitfall)

When calling the `.exe` from **PowerShell**, double quotes inside your string arguments may not be properly escaped on the command line that the executable receives. This is a known PowerShell-to-native-command issue.

**Workarounds:**

1. Escape inner double quotes with `\"`:
   ```powershell
   regex search '<sheet name=\"Database\" sheetId=\"1\"/>' 'sheetId'
   ```

2. Use PowerShell 7.3+ standard argument passing:
   ```powershell
   $PSNativeCommandArgumentPassing = 'Standard'
   regex search '<sheet name="Database" sheetId="1"/>' 'sheetId'
   ```

3. Use JSON batch mode (bypasses shell quoting entirely):
   ```powershell
   echo '[{"instruction":"search","string":"<sheet name=\"Database\" sheetId=\"1\"/>","pattern":"sheetId"}]' | regex --json
   ```

**General quoting advice:**

- **PowerShell:** Use single quotes `'...'` for both string and pattern. Single quotes are literal (no interpolation).
- **bash/zsh:** Use single quotes `'...'` for the regex. Everything inside is literal.
- **cmd.exe:** Use double quotes `"..."`. Escape inner quotes with `\"`.

### Regex syntax differences from Python

This tool uses [Rust's regex crate](https://docs.rs/regex/latest/regex/#syntax), which supports most PCRE2 syntax but has some differences from Python's `re`:

- **No lookahead/lookbehind** — the regex crate does not support `(?=...)`, `(?!...)`, `(?<=...)`, `(?<!...)` (for guaranteed linear-time matching).
- **No backreferences** — `\1`, `\2` etc. are not supported in patterns.
- **Named groups** — use `(?P<name>...)` (same as Python) or `(?<name>...)`.
- **Substitution syntax** — use `$1`, `$2`, `${name}` (not `\1`, `\g<name>` as in Python).

If you need lookahead/lookbehind or backreferences, consider using the `fancy-regex` crate instead (trade-off: slightly slower).

### Return values

- `search` returns an integer: the 0-based byte offset of the first match, or `-1` if no match.
- `findall` and `split` return JSON arrays (not Python list repr).
- An invalid regex pattern always returns `False` (printed to stdout, exit code 1).
- Exit code is `0` on success, `1` on invalid regex or usage error.

### Performance

- Startup time: ~2-5ms (vs ~300-500ms with PyInstaller, ~80-150ms with Nuitka)
- Binary size: ~1-3 MB (statically linked, stripped)
- The Rust regex crate guarantees linear-time matching — no catastrophic backtracking.
- For multiple operations, use `--json` batch mode to avoid even the tiny per-invocation overhead.

### Build optimization

The `Cargo.toml` is configured for maximum performance in release builds:
- `opt-level = 3` — full optimization
- `lto = true` — link-time optimization across all crates
- `codegen-units = 1` — single codegen unit for better optimization
- `strip = true` — strip debug symbols for smaller binary

## Project structure

```
regex_wrapper/
  Cargo.toml              # Rust package manifest
  src/
    main.rs               # The CLI tool (source)
  target/
    release/
      regex.exe           # Built executable (Windows)
  README.md               # This file
  regex_wrapper.log       # Debug log (created when REGEX_WRAPPER_DEBUG=1)
```

## License

MIT