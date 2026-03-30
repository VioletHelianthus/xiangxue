use std::io::Read;
use std::path::Path;

use crate::font::FontRegistry;
use crate::types::Backend;
use crate::{format_tree, parse_html, resolve_layout, resolve_layout_with_font};

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

    let config = load_config();
    let (dw, dh) = (config.design_width, config.design_height);
    let backend = make_backend(dw, dh);

    // Create font registry if configured
    let mut font_registry = config.font_dir.map(|dir| {
        eprintln!("  font dir: {}", dir);
        let mut reg = FontRegistry::new(&dir, &config.font_default);
        for (name, filename) in &config.font_aliases {
            reg.add_alias(name, filename);
        }
        reg
    });

    // No inputs: read from stdin (single file mode)
    if html_files.is_empty() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            std::process::exit(1);
        });
        process_single(&buf, emit_mode, &backend, font_registry.as_mut());
        return;
    }

    // Single file without -o: output to stdout
    if html_files.len() == 1 && out_dir.is_none() {
        let html = std::fs::read_to_string(&html_files[0]).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", html_files[0], e);
            std::process::exit(1);
        });
        process_single(&html, emit_mode, &backend, font_registry.as_mut());
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
        do_layout(&mut tree, dw, dh, font_registry.as_mut());
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

fn do_layout(tree: &mut crate::types::UiNode, dw: f32, dh: f32, registry: Option<&mut FontRegistry>) {
    if let Some(r) = registry {
        resolve_layout_with_font(tree, dw, dh, r);
    } else {
        resolve_layout(tree, dw, dh);
    }
}

fn process_single<B: Backend>(html: &str, emit_mode: bool, backend: &B, registry: Option<&mut FontRegistry>) {
    let mut tree = parse_html(html);
    if emit_mode {
        let (dw, dh) = backend.design_size();
        do_layout(&mut tree, dw, dh, registry);
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

struct CliConfig {
    design_width: f32,
    design_height: f32,
    font_dir: Option<String>,
    font_default: String,
    font_aliases: Vec<(String, String)>,
}

/// Load configuration from converter.toml (search upward from cwd).
/// Falls back to converter.json for backward compatibility.
fn load_config() -> CliConfig {
    let default = CliConfig {
        design_width: 640.0,
        design_height: 960.0,
        font_dir: None,
        font_default: String::new(),
        font_aliases: Vec::new(),
    };

    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        // Try converter.toml first
        let toml_path = d.join("converter.toml");
        if let Ok(content) = std::fs::read_to_string(&toml_path) {
            if let Some(cfg) = parse_toml_config(&content) {
                eprintln!("  config: {}", toml_path.display());
                return cfg;
            }
        }
        // Fallback to converter.json
        let json_path = d.join("converter.json");
        if let Ok(content) = std::fs::read_to_string(&json_path) {
            if let Some((w, h)) = parse_json_design_resolution(&content) {
                eprintln!("  config: {}", json_path.display());
                return CliConfig {
                    design_width: w,
                    design_height: h,
                    font_dir: None,
                    font_default: String::new(),
                    font_aliases: Vec::new(),
                };
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    default
}

fn parse_toml_config(content: &str) -> Option<CliConfig> {
    #[derive(serde::Deserialize)]
    struct Root {
        design: Option<Design>,
        font: Option<FontCfg>,
    }
    #[derive(serde::Deserialize)]
    struct Design {
        width: Option<f32>,
        height: Option<f32>,
    }
    #[derive(serde::Deserialize)]
    struct FontCfg {
        dir: Option<String>,
        default: Option<String>,
        alias: Option<std::collections::HashMap<String, String>>,
    }

    let root: Root = toml::from_str(content).ok()?;
    let design = root.design?;
    let font = root.font.unwrap_or(FontCfg { dir: None, default: None, alias: None });
    let aliases: Vec<(String, String)> = font.alias.unwrap_or_default().into_iter().collect();
    Some(CliConfig {
        design_width: design.width.unwrap_or(640.0),
        design_height: design.height.unwrap_or(960.0),
        font_dir: font.dir,
        font_default: font.default.unwrap_or_default(),
        font_aliases: aliases,
    })
}

/// Legacy JSON config support.
fn parse_json_design_resolution(json: &str) -> Option<(f32, f32)> {
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
    let num_end = after_colon
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(after_colon.len());
    after_colon[..num_end].parse().ok()
}
