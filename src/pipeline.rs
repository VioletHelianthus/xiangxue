use crate::box_model::Size;
use crate::document::LayoutTree;
use crate::error::LayoutError;
use crate::font::{FontProvider, NoOpFontProvider};

#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub viewport: Size,
    pub design_size: Option<Size>,
    pub root_font_size: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            viewport: Size::new(1280.0, 720.0),
            design_size: None,
            root_font_size: 16.0,
        }
    }
}

/// One-shot layout entrypoint.
///
/// **Text-measurement warning**: this entrypoint uses [`NoOpFontProvider`],
/// which approximates text width as `chars * font_size * 0.6` rather than
/// performing real font metrics. Use [`Engine::new`] with a real
/// [`FontProvider`] for production text layout; the measurements returned
/// here are only suitable for layout-only golden tests, fixtures where text
/// widths are not asserted, or quick experiments.
pub fn layout(
    html: &str,
    css: &[&str],
    opts: &LayoutOptions,
) -> Result<LayoutTree, LayoutError> {
    let engine = Engine::new(Box::new(NoOpFontProvider));
    engine.layout_with(html, css, opts)
}

pub type StylesheetId = usize;

/// Reusable layout engine with cached stylesheet ASTs and font provider.
/// Suitable for batch construction (e.g. building many pages in one CLI run).
pub struct Engine {
    pub(crate) fonts: Box<dyn FontProvider>,
    pub(crate) stylesheets: Vec<String>,
}

impl Engine {
    pub fn new(fonts: Box<dyn FontProvider>) -> Self {
        Engine { fonts, stylesheets: Vec::new() }
    }

    pub fn add_stylesheet(&mut self, css: &str) -> Result<StylesheetId, LayoutError> {
        let id = self.stylesheets.len();
        self.stylesheets.push(css.to_owned());
        Ok(id)
    }

    pub fn layout(&self, html: &str, opts: &LayoutOptions) -> Result<LayoutTree, LayoutError> {
        self.layout_with(html, &[], opts)
    }

    pub(crate) fn layout_with(
        &self,
        html: &str,
        extra_css: &[&str],
        opts: &LayoutOptions,
    ) -> Result<LayoutTree, LayoutError> {
        // Phase 1: parse HTML → arena Document.
        let mut document = crate::parse::html::parse(html)?;

        // Phase 2: cascade CSS → ComputedStyle on each element.
        crate::cascade::run(&mut document, &self.stylesheets, extra_css)?;

        // Phase 3: ComputedStyle → taffy::Style cached on each node.
        crate::layout::flush_styles(&mut document)?;

        // Phase 4: solve layout, write box_subpixel + box_pixel.
        crate::layout::solve(&mut document, opts, &*self.fonts)?;

        Ok(LayoutTree { document, viewport: opts.viewport })
    }
}
