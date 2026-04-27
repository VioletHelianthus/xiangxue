//! Leaf measurement for Text nodes.
//!
//! Taffy invokes our `compute_child_layout` on every node. For Text nodes we
//! call the active [`FontProvider`] with the parent Element's font properties
//! and return the resulting [`taffy::Size`].

use taffy::{AvailableSpace, LayoutInput, LayoutOutput, Size};

use crate::document::Document;
use crate::font::FontQuery;
use crate::node::{NodeId, NodeKind};
use crate::style::{Font, Visibility};

/// Compute layout for a Text node by measuring its content.
///
/// Caller has already verified that `id` refers to a Text node and that
/// the node is not display:none. Returns a Taffy LayoutOutput.
pub(super) fn compute_text_leaf(
    doc: &mut Document,
    id: NodeId,
    inputs: LayoutInput,
) -> LayoutOutput {
    let style = doc.get(id).map(|n| n.taffy_style.clone()).unwrap_or_default();

    // Pull the text content + the parent Element's font properties.
    let text = match doc.get(id).map(|n| &n.kind) {
        Some(NodeKind::Text(s)) => s.clone(),
        _ => return LayoutOutput::HIDDEN,
    };
    let parent_font = parent_font_for_text(doc, id);

    // visibility:hidden still occupies space (per CSS spec); display:none
    // is handled upstream by Taffy and so we never reach here for it.
    let _ = parent_font.line_height; // (read for parity; size below uses metrics width/height)

    let fonts = match doc.font_provider() {
        Some(fp) => fp,
        // Without a FontProvider, treat text as zero-size leaf. This shouldn't
        // happen in normal use because solve() always installs one.
        None => {
            return taffy::compute_leaf_layout(inputs, &style, |_, _| 0.0, |_, _| Size::ZERO);
        }
    };

    taffy::compute_leaf_layout(inputs, &style, |_, _| 0.0, |known, available| {
        let measured = measure_text(fonts, &text, &parent_font, known, available);
        Size {
            width: measured.width,
            height: measured.height,
        }
    })
}

/// Walk up to the nearest Element ancestor and return its font.
fn parent_font_for_text(doc: &Document, text_id: NodeId) -> Font {
    let mut current = doc.get(text_id).and_then(|n| n.parent);
    while let Some(id) = current {
        if let Some(node) = doc.get(id) {
            if let NodeKind::Element { computed, .. } = &node.kind {
                return computed.font.clone();
            }
            current = node.parent;
        } else {
            break;
        }
    }
    Font::default()
}

/// Run the FontProvider with the given font query and return text dimensions.
///
/// Honours Taffy's known/available constraints by clamping the natural width
/// when the available space pins it. Multi-line wrapping isn't implemented in
/// M5 — the FontProvider is asked once for the natural width/height. Callers
/// needing wrap behaviour set explicit `width` on the parent Element so Taffy
/// constrains the leaf via known dimensions.
fn measure_text(
    fonts: &dyn crate::font::FontProvider,
    text: &str,
    font: &Font,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let query = FontQuery {
        family: &font.family,
        size: font.size,
        weight: font.weight,
        style: font.style,
    };
    let metrics = fonts.measure(text, &query);

    let mut width = metrics.width;
    if let Some(known_w) = known.width {
        width = known_w.min(width);
    } else if let AvailableSpace::Definite(avail_w) = available.width {
        width = width.min(avail_w);
    }

    let mut height = metrics.height;
    if let Some(known_h) = known.height {
        height = known_h.min(height);
    }
    Size { width, height }
}

/// Returns true if `id` is an Element with visibility:hidden. Used by the
/// dispatcher to keep the box but skip painting decisions; layout still runs.
#[allow(dead_code)]
pub(super) fn is_visibility_hidden(doc: &Document, id: NodeId) -> bool {
    matches!(
        doc.get(id).map(|n| &n.kind),
        Some(NodeKind::Element { computed, .. }) if computed.visibility == Visibility::Hidden
    )
}
