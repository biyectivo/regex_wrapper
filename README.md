# regex_wrapper

A command-line wrapper around Python 3.14's `re` module. Provides regex operations (`search`, `findall`, `split`, `sub`) callable directly from the shell, with support for both single-command CLI usage and JSON batch mode for processing multiple operations in one invocation.

## Features

- Direct CLI access to Python's regex engine
- JSON batch mode for multiple operations without repeated process startup
- Compiles to a single-file native executable via Nuitka (Windows, Linux, macOS)
- Optional debug logging (timestamped args + results)
- Invalid regex patterns return `False` instead of raising exceptions

## Requirements

- Python 3.14+
- [Nuitka](https://nuitka.net/) (for building the executable): `pip install nuitka`
- A C compiler (for Nuitka):
  - **Windows:** MSVC (Visual Studio) or MinGW64 (Nuitka auto-downloads if needed)
  - **Linux:** `gcc` or `clang` (`apt install gcc` / `dnf install gcc`)
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)

## Building

```bash
pip install nuitka
python build.py
```

The executable is output to `dist/regex.exe` (Windows) or `dist/regex` (Linux/macOS).

You can explicitly specify the target OS (must match the machine you're building on — no cross-compilation):

```bash
python build.py --os windows
python build.py --os linux
python build.py --os macos
```

After building, copy the executable to a directory on your `PATH`.

## Usage

### CLI mode

```
regex <instruction> <string> <pattern> [<flags>]
regex sub <string> <pattern> <replacement> [<flags>]
```

**Instructions:**

| Instruction | Description | Output |
|---|---|---|
| `search` | Find first match | Start index (>= 0) or `-1` if no match |
| `findall` | Find all matches | Python list of matches |
| `split` | Split string by pattern | Python list of parts |
| `sub` | Substitute matches | Resulting string |

**Flags** (optional, 4th or 5th argument):

- Shorthand letters: `i`, `m`, `s`, `x`, `a`, `u` (e.g. `"im"`)
- Named: `IGNORECASE|MULTILINE`
- Prefixed: `re.IGNORECASE|re.MULTILINE`

**Examples:**

```bash
regex search "hello world" "\w+"
# Output: 0

regex search "hello world" "xyz"
# Output: -1

regex findall "one 1 two 2 three 3" "\d+"
# Output: ['1', '2', '3']

regex split "one,two,,three" ","
# Output: ['one', 'two', '', 'three']

regex sub "hello world" "world" "Python"
# Output: hello Python

regex search "hello world" "\w+" i
# Output: 0

regex search "test" "[invalid"
# Output: False
```

### JSON batch mode

Pipe a JSON array to stdin with the `--json` flag. Each element is an object describing one operation. Returns a JSON array of results on stdout.

```bash
echo '[{"instruction":"search","string":"hello world","pattern":"\\w+"},{"instruction":"findall","string":"one 1 two 2","pattern":"\\d+"}]' | regex --json
```

Output:
```json
[0, ["1", "2"]]
```

**Entry format:**

```json
{
  "instruction": "search|findall|split|sub",
  "string": "target string",
  "pattern": "regex pattern",
  "flags": "optional flags string",
  "replacement": "required for sub"
}
```

**Error handling in batch mode:**

- Invalid regex returns `false` for that entry
- Unknown instruction returns `{"error": "..."}`
- Invalid JSON on stdin prints an error to stderr and exits with code 1

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
[2026-06-14 21:51:12.339] args: 'search' 'hello' '\\w+' | result: 0
[2026-06-14 21:51:12.500] batch: {"instruction": "search", ...} | result: 0
```

## Gotchas, tips, and caveats

### Shell quoting (the biggest pitfall)

When calling the compiled `.exe` from **PowerShell**, double quotes inside your string arguments are not properly escaped on the command line that the executable receives. This is a known PowerShell-to-native-command issue.

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

### Backslash count

The regex `\d` (match a digit) only needs one layer of escaping since the shell (with single quotes) passes it verbatim:

| Shell | You type | Python `re` receives |
|---|---|---|
| PowerShell `'...'` | `'\d+'` | `\d+` |
| bash `'...'` | `'\d+'` | `\d+` |
| cmd.exe `"..."` | `"\d+"` | `\d+` |

For a **literal backslash** match (regex `\\`):

| Shell | You type | Python `re` receives |
|---|---|---|
| PowerShell `'...'` | `'\\'` | `\\` |
| bash `'...'` | `'\\'` | `\\` |

### Return values

- `search` returns an integer: the 0-based start index of the first match, or `-1` if no match.
- An invalid regex pattern always returns `False` (not an exception).
- Exit code is `0` on success, `1` on invalid regex or usage error.

### Performance

- The Nuitka-compiled executable starts in ~80-150ms (vs ~300-500ms with PyInstaller).
- For repeated calls, use `--json` batch mode to avoid paying startup cost per operation.
- If sub-10ms startup is required, consider rewriting in Rust or Go.

### Cross-compilation

Nuitka (like PyInstaller) **cannot cross-compile**. You must run `build.py` on the target OS. For multi-platform builds, use CI (e.g. GitHub Actions with a matrix of `windows-latest`, `ubuntu-latest`, `macos-latest`).

## Project structure

```
regex_wrapper/
  regex.py              # The CLI tool (source)
  build.py              # Build script (Nuitka)
  README.md             # This file
  dist/                 # Build output (regex.exe or regex)
  regex_wrapper.log     # Debug log (created when REGEX_WRAPPER_DEBUG=1)
```

## License

MIT
