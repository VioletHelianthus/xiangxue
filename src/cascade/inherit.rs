//! Inherit inheritable properties from parent ComputedStyle to child.
//!
//! W3C lists inheritable CSS properties — we only handle the ones in our
//! subset (`docs/xiangxue-css-subset.md`). This is run as a separate pass
//! after the cascade pass writes initial values into each Element node.

use crate::style::ComputedStyle;

/// Mutate `child` so any inheritable property still at its initial value
/// adopts the parent's value. Behaviour mirrors CSS: applies only to
/// inheritable properties, and only when the child didn't explicitly set them.
///
/// In v2's first cut we use a coarse "if value == default, take parent's"
/// heuristic. This is correct for most CSS uses (font-size: 16px is the
/// default; once it changes, the change propagates). A full implementation
/// would track "explicit" vs "default" via Option<T> — leaving as a future
/// refinement noted in `docs/xiangxue-css-subset.md` §13.
pub fn inherit_from(child: &mut ComputedStyle, parent: &ComputedStyle) {
    let initial = ComputedStyle::initial();

    // color
    if computed_color_eq(&child.color, &initial.color) {
        child.color = parent.color.clone();
    }

    // font-* (size/family/weight/style/line-height)
    if child.font.family == initial.font.family {
        child.font.family = parent.font.family.clone();
    }
    if (child.font.size - initial.font.size).abs() < f32::EPSILON {
        child.font.size = parent.font.size;
    }
    if child.font.weight == initial.font.weight {
        child.font.weight = parent.font.weight;
    }
    if child.font.style == initial.font.style {
        child.font.style = parent.font.style;
    }
    if (child.font.line_height - initial.font.line_height).abs() < f32::EPSILON {
        child.font.line_height = parent.font.line_height;
    }

    // text-align
    if child.text_align == initial.text_align {
        child.text_align = parent.text_align;
    }

    // visibility (inherited per CSS spec)
    if child.visibility == initial.visibility {
        child.visibility = parent.visibility;
    }
}

fn computed_color_eq(a: &crate::style::Color, b: &crate::style::Color) -> bool {
    use crate::style::Color::*;
    match (a, b) {
        (Rgba(r1, g1, b1, a1), Rgba(r2, g2, b2, a2)) => {
            r1 == r2 && g1 == g2 && b1 == b2 && a1 == a2
        }
        (CurrentColor, CurrentColor) => true,
        _ => false,
    }
}
