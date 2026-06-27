use chrono::Local;
use regex::Regex;
use serde_json::{json, Value};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_slice = &args[1..];

    if args_slice.is_empty() {
        print_usage();
        process::exit(1);
    }

    // JSON batch mode
    if args_slice[0] == "--json" {
        let input_file = args_slice.get(1).map(|s| s.as_str());
        let output_file = args_slice.get(2).map(|s| s.as_str());
        run_batch(input_file, output_file);
        return;
    }

    if args_slice.len() < 3 {
        print_usage();
        process::exit(1);
    }

    let instruction = args_slice[0].to_lowercase();
    let valid = ["search", "findall", "split", "sub"];
    if !valid.contains(&instruction.as_str()) {
        eprintln!(
            "Unknown instruction: {:?}  (expected one of {:?})",
            instruction, valid
        );
        process::exit(1);
    }

    let string = &args_slice[1];
    let pattern = &args_slice[2];

    // For "sub" we need an extra <replacement> argument before optional flags
    let (replacement, flags_str) = if instruction == "sub" {
        if args_slice.len() < 4 {
            eprintln!("'sub' requires a <replacement> argument after <pattern>.");
            process::exit(1);
        }
        let repl = &args_slice[3];
        let flags = if args_slice.len() > 4 {
            &args_slice[4]
        } else {
            ""
        };
        (repl.as_str(), flags)
    } else {
        let flags = if args_slice.len() > 3 {
            args_slice[3].as_str()
        } else {
            ""
        };
        ("", flags)
    };

    let flags = match parse_flags(flags_str) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Compile the pattern — invalid regex returns "False"
    let compiled = match build_regex(pattern, &flags) {
        Some(r) => r,
        None => {
            println!("False");
            debug_log_cli(args_slice, "False");
            process::exit(1);
        }
    };

    // Dispatch
    match instruction.as_str() {
        "search" => {
            if let Some(m) = compiled.find(string) {
                let result = m.start().to_string();
                println!("{}", result);
                debug_log_cli(args_slice, &result);
            } else {
                println!("-1");
                debug_log_cli(args_slice, "-1");
            }
        }
        "findall" => {
            let result = findall_python_style(&compiled, string);
            println!("{}", result);
            debug_log_cli(args_slice, &result);
        }
        "split" => {
            let parts: Vec<&str> = compiled.split(string).collect();
            let result = format!("{:?}", parts);
            println!("{}", result);
            debug_log_cli(args_slice, &result);
        }
        "sub" => {
            let result = compiled.replace_all(string, replacement).to_string();
            println!("{}", result);
            debug_log_cli(args_slice, &result);
        }
        _ => unreachable!(),
    }
}

/// Flags parsed from the flag string, used to build the regex pattern with inline flags.
struct RegexFlags {
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_newline: bool,
    verbose: bool,
    unicode: bool,
}

impl RegexFlags {
    fn new() -> Self {
        Self {
            case_insensitive: false,
            multi_line: false,
            dot_matches_newline: false,
            verbose: false,
            unicode: true, // Rust regex is Unicode by default
        }
    }

    /// Build the inline flag prefix string for the regex, e.g. "(?ims)"
    fn to_prefix(&self) -> String {
        let mut flags = String::new();
        if self.case_insensitive {
            flags.push('i');
        }
        if self.multi_line {
            flags.push('m');
        }
        if self.dot_matches_newline {
            flags.push('s');
        }
        if self.verbose {
            flags.push('x');
        }
        if !self.unicode {
            // Disable unicode with (?-u)
            if flags.is_empty() {
                return "(?-u)".to_string();
            } else {
                return format!("(?{}-u)", flags);
            }
        }
        if flags.is_empty() {
            String::new()
        } else {
            format!("(?{})", flags)
        }
    }
}

fn parse_flags(flag_str: &str) -> Result<RegexFlags, String> {
    let mut flags = RegexFlags::new();
    if flag_str.is_empty() {
        return Ok(flags);
    }

    // Shorthand single-letter map
    let shorthand_chars = "imsxau";

    // If every character is a known shorthand letter, treat as shorthand
    if flag_str.chars().all(|c| shorthand_chars.contains(c)) {
        for ch in flag_str.chars() {
            match ch {
                'i' => flags.case_insensitive = true,
                'm' => flags.multi_line = true,
                's' => flags.dot_matches_newline = true,
                'x' => flags.verbose = true,
                'a' => flags.unicode = false, // ASCII mode = disable unicode
                'u' => flags.unicode = true,
                _ => unreachable!(),
            }
        }
        return Ok(flags);
    }

    // Otherwise parse "re.IGNORECASE|re.DOTALL" or "IGNORECASE|DOTALL"
    for token in flag_str.split('|') {
        let token = token.trim().strip_prefix("re.").unwrap_or(token.trim());
        match token {
            "IGNORECASE" => flags.case_insensitive = true,
            "MULTILINE" => flags.multi_line = true,
            "DOTALL" => flags.dot_matches_newline = true,
            "VERBOSE" => flags.verbose = true,
            "ASCII" => flags.unicode = false,
            "UNICODE" => flags.unicode = true,
            _ => return Err(format!("Unknown flag: {}", token)),
        }
    }
    Ok(flags)
}

fn build_regex(pattern: &str, flags: &RegexFlags) -> Option<Regex> {
    let prefix = flags.to_prefix();
    let full_pattern = format!("{}{}", prefix, pattern);
    Regex::new(&full_pattern).ok()
}

/// Python-style findall: if no capture groups, return full matches.
/// If one capture group, return a flat list of that group's matches.
/// If multiple capture groups, return a list of arrays (one per match).
fn findall_python_style(re: &Regex, string: &str) -> String {
    let num_groups = re.captures_len() - 1; // captures_len() includes group 0

    if num_groups == 0 {
        let matches: Vec<&str> = re.find_iter(string).map(|m| m.as_str()).collect();
        format!("{:?}", matches)
    } else if num_groups == 1 {
        let matches: Vec<&str> = re
            .captures_iter(string)
            .filter_map(|caps| caps.get(1).map(|m| m.as_str()))
            .collect();
        format!("{:?}", matches)
    } else {
        let matches: Vec<Vec<&str>> = re
            .captures_iter(string)
            .map(|caps| {
                (1..=num_groups)
                    .map(|i| caps.get(i).map(|m| m.as_str()).unwrap_or(""))
                    .collect()
            })
            .collect();
        format!("{:?}", matches)
    }
}

/// Same logic as above but returns a serde_json::Value for batch mode.
fn findall_python_style_json(re: &Regex, string: &str) -> Value {
    let num_groups = re.captures_len() - 1;

    if num_groups == 0 {
        let matches: Vec<&str> = re.find_iter(string).map(|m| m.as_str()).collect();
        json!(matches)
    } else if num_groups == 1 {
        let matches: Vec<&str> = re
            .captures_iter(string)
            .filter_map(|caps| caps.get(1).map(|m| m.as_str()))
            .collect();
        json!(matches)
    } else {
        let matches: Vec<Vec<&str>> = re
            .captures_iter(string)
            .map(|caps| {
                (1..=num_groups)
                    .map(|i| caps.get(i).map(|m| m.as_str()).unwrap_or(""))
                    .collect()
            })
            .collect();
        json!(matches)
    }
}

// --- JSON batch mode ---

fn run_batch(input_file: Option<&str>, output_file: Option<&str>) {
    // Read input from file or stdin
    let input = if let Some(path) = input_file {
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Error reading file {:?}: {}", path, e);
            process::exit(1);
        })
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        });
        buf
    };

    let commands: Vec<Value> = match serde_json::from_str(&input) {
        Ok(Value::Array(arr)) => arr,
        Ok(_) => {
            eprintln!("{}", json!({"error": "Expected a JSON array"}));
            process::exit(1);
        }
        Err(e) => {
            eprintln!("{}", json!({"error": format!("Invalid JSON: {}", e)}));
            process::exit(1);
        }
    };

    let mut results: Vec<Value> = Vec::with_capacity(commands.len());
    for entry in &commands {
        let result = execute_one(entry);
        debug_log_batch(entry, &result);
        results.push(result);
    }

    let output = serde_json::to_string(&results).unwrap();

    // Write output to file or stdout
    if let Some(path) = output_file {
        std::fs::write(path, &output).unwrap_or_else(|e| {
            eprintln!("Error writing file {:?}: {}", path, e);
            process::exit(1);
        });
    } else {
        println!("{}", output);
    }
}

fn execute_one(entry: &Value) -> Value {
    let instruction = entry
        .get("instruction")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let string = entry
        .get("string")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let pattern = entry
        .get("pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let flags_str = entry
        .get("flags")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let replacement = entry
        .get("replacement")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let valid = ["search", "findall", "split", "sub"];
    if !valid.contains(&instruction.as_str()) {
        return json!({"error": format!("Unknown instruction: {:?}", instruction)});
    }

    let flags = match parse_flags(flags_str) {
        Ok(f) => f,
        Err(e) => return json!({"error": e}),
    };

    let compiled = match build_regex(pattern, &flags) {
        Some(r) => r,
        None => return json!(false),
    };

    match instruction.as_str() {
        "search" => {
            if let Some(m) = compiled.find(string) {
                json!(m.start())
            } else {
                json!(-1)
            }
        }
        "findall" => {
            findall_python_style_json(&compiled, string)
        }
        "split" => {
            let parts: Vec<&str> = compiled.split(string).collect();
            json!(parts)
        }
        "sub" => {
            let result = compiled.replace_all(string, replacement).to_string();
            json!(result)
        }
        _ => unreachable!(),
    }
}

// --- Debug logging ---

fn is_debug() -> bool {
    env::var("REGEX_WRAPPER_DEBUG")
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn log_file_path() -> PathBuf {
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    exe_dir.join("regex_wrapper.log")
}

fn debug_log_cli(args: &[String], result: &str) {
    if !is_debug() {
        return;
    }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let args_str: Vec<String> = args.iter().map(|a| format!("{:?}", a)).collect();
    let line = format!(
        "[{}] args: {} | result: {}\n",
        timestamp,
        args_str.join(" "),
        result
    );
    append_to_log(&line);
}

fn debug_log_batch(entry: &Value, result: &Value) {
    if !is_debug() {
        return;
    }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let line = format!(
        "[{}] batch: {} | result: {}\n",
        timestamp,
        serde_json::to_string(entry).unwrap_or_default(),
        serde_json::to_string(result).unwrap_or_default()
    );
    append_to_log(&line);
}

fn append_to_log(line: &str) {
    let path = log_file_path();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn print_usage() {
    eprintln!(
        "regex - command-line wrapper around Rust's regex engine.

Usage:
    regex search   <string> <pattern> [<flags>]
    regex findall  <string> <pattern> [<flags>]
    regex split    <string> <pattern> [<flags>]
    regex sub      <string> <pattern> <replacement> [<flags>]

    regex --json                          (read from stdin, write to stdout)
    regex --json <input_file>             (read from file, write to stdout)
    regex --json <input_file> <output_file>  (read from file, write to file)

Exit codes:
    0  success
    1  invalid regex or usage error"
    );
}
