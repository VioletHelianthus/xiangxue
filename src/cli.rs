use std::io::Read;
use std::path::Path;

use crate::types::Backend;
use crate::{format_tree, parse_html, resolve_layout};

/// Run the CLI with the given backend factory.
///
/// Backend crates only need:
/// ```ignore
/// fn main() {
///     xiangxue::run_cli(|w, h| MyBackend::new(w, h));
/// }
/// ```
pub fn run_cli<B, E>(make_backend: impl Fn(f32, f32) -> B)
where
    B: Backend<Error = E>,
    E: std::fmt::Display,
{
    let args: Vec<String> = std::env::args().collect();
    let emit_mode = args.iter().any(|a| a == "--emit");

    // Collect -o value if present
    let out_dir = args.windows(2).find_map(|pair| {
        if pair[0] == "-o" { Some(pair[1].clone()) } else { None }
    });

    // Collect input files (skip flags and -o value)
    let mut skip_next = false;
    let input_files: Vec<&str> = args.iter().skip(1).filter(|a| {
        if skip_next { skip_next = false; return false; }
        if *a == "-o" { skip_next = true; return false; }
        !a.starts_with("--")
    }).map(|s| s.as_str()).collect();

    // Expand directory inputs to .html files
    let mut html_files: Vec<String> = Vec::new();
    for input in &input_files {
        let path = Path::new(input);
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |e| e == "html") {
                        html_files.push(p.to_string_lossy().to_string());
                    }
                }
            }
            html_files.sort();
        } else {
            html_files.push(input.to_string());
        }
    }

    let (dw, dh) = load_design_resolution();
    let backend = make_backend(dw, dh);

    // No inputs: read from stdin (single file mode)
    if html_files.is_empty() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            std::process::exit(1);
        });
        process_single(&buf, emit_mode, &backend);
        return;
    }

    // Single file without -o: output to stdout
    if html_files.len() == 1 && out_dir.is_none() {
        let html = std::fs::read_to_string(&html_files[0]).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", html_files[0], e);
            std::process::exit(1);
        });
        process_single(&html, emit_mode, &backend);
        return;
    }

    // Batch mode: multiple files or -o specified
    if !emit_mode {
        eprintln!("Batch mode requires --emit");
        std::process::exit(1);
    }

    let out = out_dir.unwrap_or_else(|| {
        eprintln!("Multiple files require -o <outdir>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(&out).unwrap_or_else(|e| {
        eprintln!("Error creating output directory {}: {}", out, e);
        std::process::exit(1);
    });

    let (dw, dh) = backend.design_size();
    let ext = backend.extension();

    for file in &html_files {
        let html = std::fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", file, e);
            std::process::exit(1);
        });

        let mut tree = parse_html(&html);
        resolve_layout(&mut tree, dw, dh);
        let output = backend.emit(&tree).unwrap_or_else(|e| {
            eprintln!("Error converting {}: {}", file, e);
            std::process::exit(1);
        });

        let stem = Path::new(file).file_stem().unwrap().to_string_lossy();
        let out_path = Path::new(&out).join(format!("{}.{}", stem, ext));
        std::fs::write(&out_path, &output).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {}", out_path.display(), e);
            std::process::exit(1);
        });
        eprintln!("  {} → {}", Path::new(file).file_name().unwrap().to_string_lossy(), out_path.display());
    }
}

fn process_single<B: Backend>(html: &str, emit_mode: bool, backend: &B) {
    let mut tree = parse_html(html);
    if emit_mode {
        let (dw, dh) = backend.design_size();
        resolve_layout(&mut tree, dw, dh);
        match backend.emit(&tree) {
            Ok(bytes) => {
                use std::io::Write;
                std::io::stdout().write_all(&bytes).unwrap();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        print!("{}", format_tree(&tree));
    }
}

/// Parse design resolution from converter.json.
fn load_design_resolution() -> (f32, f32) {
    let default = (640.0, 960.0);

    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let config_path = d.join("converter.json");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Some(res) = parse_design_resolution(&content) {
                eprintln!("  config: {}", config_path.display());
                return res;
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    default
}

fn parse_design_resolution(json: &str) -> Option<(f32, f32)> {
    let dr_start = json.find("\"designResolution\"")?;
    let brace_start = json[dr_start..].find('{')? + dr_start;
    let brace_end = json[brace_start..].find('}')? + brace_start;
    let block = &json[brace_start..=brace_end];
    let width = extract_number(block, "width")?;
    let height = extract_number(block, "height")?;
    Some((width, height))
}

fn extract_number(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\"", key);
    let key_pos = json.find(&pattern)?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let num_end = after_colon.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(after_colon.len());
    after_colon[..num_end].parse().ok()
}
