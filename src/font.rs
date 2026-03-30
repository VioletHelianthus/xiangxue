use font_kit::font::Font;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Registry that loads fonts from a directory by name.
///
/// Convention: font name "华康圆体" → `{dir}/华康圆体.ttf` (or .otf).
/// Supports aliases: "华康圆体" → "hkyt" → `{dir}/hkyt.ttf`.
/// Fonts are loaded lazily on first use and cached.
pub struct FontRegistry {
    dir: PathBuf,
    default_name: String,
    aliases: HashMap<String, String>,
    cache: HashMap<String, Option<Font>>,
}

impl FontRegistry {
    pub fn new(dir: impl Into<PathBuf>, default_name: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            default_name: default_name.into(),
            aliases: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Add an alias mapping: font name → ttf filename (without extension).
    pub fn add_alias(&mut self, name: impl Into<String>, filename: impl Into<String>) {
        self.aliases.insert(name.into(), filename.into());
    }

    /// Measure text using the named font at the given size.
    /// Falls back to default font, then to heuristic estimation.
    pub fn measure_text(&mut self, text: &str, font_name: &str, font_size: f32) -> (f32, f32) {
        if let Some(font) = self.load_font(font_name) {
            return measure_with_font(font, text, font_size);
        }
        // Fallback to default font
        if font_name != self.default_name {
            let default = self.default_name.clone();
            if let Some(font) = self.load_font(&default) {
                return measure_with_font(font, text, font_size);
            }
        }
        // Last resort: heuristic
        fallback_measure(text, font_size)
    }

    /// Load a font by name, trying alias then direct name. Caches the result.
    fn load_font(&mut self, name: &str) -> Option<&Font> {
        if !self.cache.contains_key(name) {
            let font = if let Some(alias) = self.aliases.get(name) {
                try_load_font(&self.dir, alias)
            } else {
                None
            }
            .or_else(|| try_load_font(&self.dir, name));
            self.cache.insert(name.to_string(), font);
        }
        self.cache.get(name).and_then(|f| f.as_ref())
    }
}

fn try_load_font(dir: &Path, name: &str) -> Option<Font> {
    for ext in &["ttf", "otf", "TTF", "OTF"] {
        let path = dir.join(format!("{}.{}", name, ext));
        if let Ok(font) = Font::from_path(&path, 0) {
            return Some(font);
        }
    }
    None
}

fn measure_with_font(font: &Font, text: &str, font_size: f32) -> (f32, f32) {
    let metrics = font.metrics();
    let scale = font_size / metrics.units_per_em as f32;

    let mut width = 0.0f32;
    for ch in text.chars() {
        if let Some(glyph_id) = font.glyph_for_char(ch) {
            if let Ok(advance) = font.advance(glyph_id) {
                width += advance.x() * scale;
            } else {
                width += fallback_char_width(ch, font_size);
            }
        } else {
            width += fallback_char_width(ch, font_size);
        }
    }

    let height = (metrics.ascent - metrics.descent) * scale;
    (width.ceil(), height.ceil())
}

fn fallback_measure(text: &str, font_size: f32) -> (f32, f32) {
    let width: f32 = text.chars().map(|c| fallback_char_width(c, font_size)).sum();
    (width.ceil(), font_size.ceil())
}

fn fallback_char_width(ch: char, font_size: f32) -> f32 {
    if ch.is_ascii() {
        font_size * 0.5
    } else {
        font_size
    }
}

/// Parse a font specifier string `"{name}{size}"` into (name, size).
///
/// Examples: "MyFont16" → ("MyFont", 16.0), "宋体14" → ("宋体", 14.0).
/// If no trailing digits, size defaults to 14.0.
pub fn parse_font_spec(spec: &str) -> (String, f32) {
    // Split trailing digits
    let digit_start = spec
        .rfind(|c: char| !c.is_ascii_digit())
        .map(|i| i + spec[i..].chars().next().unwrap().len_utf8())
        .unwrap_or(0);

    let name_part = &spec[..digit_start];
    let size_part = &spec[digit_start..];
    let size = size_part.parse::<f32>().unwrap_or(14.0);

    (name_part.to_string(), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_font() {
        let (name, size) = parse_font_spec("华康圆体16");
        assert_eq!(name, "华康圆体");
        assert_eq!(size, 16.0);
    }

    #[test]
    fn parse_songti() {
        let (name, size) = parse_font_spec("宋体14");
        assert_eq!(name, "宋体");
        assert_eq!(size, 14.0);
    }

    #[test]
    fn parse_with_prefix() {
        // Core does not strip prefixes — that's engine-specific
        let (name, size) = parse_font_spec("带边华康圆体12");
        assert_eq!(name, "带边华康圆体");
        assert_eq!(size, 12.0);
    }

    #[test]
    fn parse_no_size() {
        let (name, size) = parse_font_spec("华康圆体");
        assert_eq!(name, "华康圆体");
        assert_eq!(size, 14.0); // default
    }
}
