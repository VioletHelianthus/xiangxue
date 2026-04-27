use crate::box_model::Size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Normal,
    Bold,
    Weight(u16),
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone)]
pub struct FontQuery<'a> {
    pub family: &'a str,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
}

impl TextMetrics {
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

/// Backend-supplied font measurement. xiangxue does not load fonts itself.
///
/// Not bound by `Send + Sync` so backends are free to use platform-native
/// font handles that aren't thread-safe (e.g. font-kit's directwrite loader
/// on Windows holds non-Send COM pointers). Layout itself is single-threaded
/// per Document, so this is sufficient.
pub trait FontProvider {
    fn measure(&self, text: &str, query: &FontQuery<'_>) -> TextMetrics;
    fn has_face(&self, family: &str, weight: FontWeight, style: FontStyle) -> bool;
}

/// Stub `FontProvider` returning rough estimates: width = chars × size × 0.6,
/// height = size × 1.2. Used by [`crate::layout`] when no real provider is
/// supplied — useful when text widths aren't asserted (layout-only golden
/// tests, simple fixtures), but **not suitable for production text layout**.
/// Production callers should pass a real `FontProvider` through
/// [`crate::Engine::new`].
pub struct NoOpFontProvider;

impl FontProvider for NoOpFontProvider {
    fn measure(&self, text: &str, query: &FontQuery<'_>) -> TextMetrics {
        let width = text.chars().count() as f32 * query.size * 0.6;
        let height = query.size * 1.2;
        TextMetrics {
            width,
            height,
            ascent: query.size,
            descent: query.size * 0.2,
        }
    }

    fn has_face(&self, _family: &str, _weight: FontWeight, _style: FontStyle) -> bool {
        true
    }
}
