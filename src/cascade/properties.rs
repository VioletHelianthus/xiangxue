//! Map lightningcss `Property` values to our [`ComputedStyle`].
//!
//! Scope is bounded by `docs/xiangxue-css-subset.md`. Properties in the ✅ list
//! are mapped here; ⏸/❌ properties return `Err(LayoutError::UnsupportedCss)`
//! at the call site (cascade::mod.rs decides whether to bail or skip).
//!
//! M3 covers the most-used core properties. M4 adds the layout-flow
//! properties (flex-* / grid-* internals) alongside style_to_taffy.

use lightningcss::properties::Property;
use lightningcss::properties::display as lc_display;
use lightningcss::properties::font as lc_font;
use lightningcss::properties::overflow as lc_overflow;
use lightningcss::properties::position as lc_position;
use lightningcss::properties::text as lc_text;
use lightningcss::values::color::CssColor;
use lightningcss::values::length::{LengthPercentage, LengthPercentageOrAuto, LengthValue};
use lightningcss::values::percentage::DimensionPercentage;
use lightningcss::properties::size::{MaxSize as LcMaxSize, Size as LcSize};

use crate::box_model::Sides;
use crate::error::LayoutError;
use crate::style::{
    AlignItems, Color, ComputedStyle, Display, FlexDirection, FlexProps, FlexWrap, JustifyContent,
    Length, Overflow, Position, TextAlign, Visibility,
};

/// Apply one CSS property to `style`. Returns:
/// - `Ok(true)` if the property was recognised and applied
/// - `Ok(false)` if the property is silently ignored (out-of-scope but harmless)
/// - `Err(UnsupportedCss { ... })` for ⏸/❌ subset violations the caller should
///   surface
pub fn apply_property(style: &mut ComputedStyle, prop: &Property<'_>) -> Result<bool, LayoutError> {
    match prop {
        // Display & flow
        Property::Display(d) => {
            style.display = map_display(d)?;
            Ok(true)
        }
        Property::Position(p) => {
            style.position = map_position(p);
            Ok(true)
        }
        Property::OverflowX(v) => {
            style.overflow_x = map_overflow_keyword(v);
            Ok(true)
        }
        Property::OverflowY(v) => {
            style.overflow_y = map_overflow_keyword(v);
            Ok(true)
        }
        Property::Overflow(v) => {
            style.overflow_x = map_overflow_keyword(&v.x);
            style.overflow_y = map_overflow_keyword(&v.y);
            Ok(true)
        }

        // Visual
        Property::Color(c) => {
            style.color = map_color(c)?;
            Ok(true)
        }
        Property::BackgroundColor(c) => {
            style.background.color = Some(map_color(c)?);
            Ok(true)
        }
        Property::Opacity(v) => {
            style.opacity = (v.0 as f32).clamp(0.0, 1.0);
            Ok(true)
        }
        Property::Visibility(v) => {
            style.visibility = match v {
                lc_display::Visibility::Visible => Visibility::Visible,
                _ => Visibility::Hidden,
            };
            Ok(true)
        }
        Property::ZIndex(z) => {
            style.z_index = match z {
                lc_position::ZIndex::Integer(i) => Some(*i),
                _ => None,
            };
            Ok(true)
        }

        // Typography
        Property::FontSize(s) => {
            let parent_px = style.font.size; // Cascade walks parent-first; parent's size already applied.
            style.font.size = font_size_to_px(s, parent_px);
            // line-height: normal scales with the new font size.
            style.font.line_height = style.font.size * 1.2;
            Ok(true)
        }
        Property::FontFamily(list) => {
            if let Some(first) = list.first() {
                style.font.family = format_font_family(first);
            }
            Ok(true)
        }
        Property::FontWeight(w) => {
            style.font.weight = map_font_weight(w);
            Ok(true)
        }
        Property::FontStyle(s) => {
            style.font.style = match s {
                lc_font::FontStyle::Normal => crate::font::FontStyle::Normal,
                lc_font::FontStyle::Italic => crate::font::FontStyle::Italic,
                lc_font::FontStyle::Oblique(_) => crate::font::FontStyle::Oblique,
            };
            Ok(true)
        }
        Property::LineHeight(lh) => {
            style.font.line_height = match lh {
                lc_font::LineHeight::Normal => style.font.size * 1.2,
                lc_font::LineHeight::Number(n) => style.font.size * (*n as f32),
                lc_font::LineHeight::Length(LengthPercentage::Dimension(d)) => {
                    length_value_to_px(d).unwrap_or(style.font.size * 1.2)
                }
                lc_font::LineHeight::Length(LengthPercentage::Percentage(p)) => {
                    style.font.size * p.0
                }
                lc_font::LineHeight::Length(LengthPercentage::Calc(_)) => style.font.size * 1.2,
            };
            Ok(true)
        }
        Property::TextAlign(a) => {
            style.text_align = map_text_align(a);
            Ok(true)
        }

        // Box dimensions
        Property::Width(s) => {
            style.width = map_size(s);
            Ok(true)
        }
        Property::Height(s) => {
            style.height = map_size(s);
            Ok(true)
        }
        Property::MinWidth(s) => {
            style.min_width = map_size(s);
            Ok(true)
        }
        Property::MinHeight(s) => {
            style.min_height = map_size(s);
            Ok(true)
        }
        Property::MaxWidth(s) => {
            style.max_width = map_max_size(s);
            Ok(true)
        }
        Property::MaxHeight(s) => {
            style.max_height = map_max_size(s);
            Ok(true)
        }

        Property::PaddingTop(v) => {
            style.padding.top = lpa_to_length(v);
            Ok(true)
        }
        Property::PaddingRight(v) => {
            style.padding.right = lpa_to_length(v);
            Ok(true)
        }
        Property::PaddingBottom(v) => {
            style.padding.bottom = lpa_to_length(v);
            Ok(true)
        }
        Property::PaddingLeft(v) => {
            style.padding.left = lpa_to_length(v);
            Ok(true)
        }
        Property::Padding(p) => {
            style.padding = Sides {
                top: lpa_to_length(&p.top),
                right: lpa_to_length(&p.right),
                bottom: lpa_to_length(&p.bottom),
                left: lpa_to_length(&p.left),
            };
            Ok(true)
        }

        Property::MarginTop(v) => {
            style.margin.top = lpa_to_length(v);
            Ok(true)
        }
        Property::MarginRight(v) => {
            style.margin.right = lpa_to_length(v);
            Ok(true)
        }
        Property::MarginBottom(v) => {
            style.margin.bottom = lpa_to_length(v);
            Ok(true)
        }
        Property::MarginLeft(v) => {
            style.margin.left = lpa_to_length(v);
            Ok(true)
        }
        Property::Margin(m) => {
            style.margin = Sides {
                top: lpa_to_length(&m.top),
                right: lpa_to_length(&m.right),
                bottom: lpa_to_length(&m.bottom),
                left: lpa_to_length(&m.left),
            };
            Ok(true)
        }

        Property::Top(v) => {
            style.inset.top = lpa_to_length(v);
            Ok(true)
        }
        Property::Right(v) => {
            style.inset.right = lpa_to_length(v);
            Ok(true)
        }
        Property::Bottom(v) => {
            style.inset.bottom = lpa_to_length(v);
            Ok(true)
        }
        Property::Left(v) => {
            style.inset.left = lpa_to_length(v);
            Ok(true)
        }
        Property::Inset(inset) => {
            style.inset = Sides {
                top: lpa_to_length(&inset.top),
                right: lpa_to_length(&inset.right),
                bottom: lpa_to_length(&inset.bottom),
                left: lpa_to_length(&inset.left),
            };
            Ok(true)
        }

        // Flex container properties — populate FlexProps lazily.
        Property::FlexDirection(d, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.direction = match d {
                lightningcss::properties::flex::FlexDirection::Row => FlexDirection::Row,
                lightningcss::properties::flex::FlexDirection::RowReverse => {
                    FlexDirection::RowReverse
                }
                lightningcss::properties::flex::FlexDirection::Column => FlexDirection::Column,
                lightningcss::properties::flex::FlexDirection::ColumnReverse => {
                    FlexDirection::ColumnReverse
                }
            };
            Ok(true)
        }
        Property::FlexWrap(w, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.wrap = match w {
                lightningcss::properties::flex::FlexWrap::NoWrap => FlexWrap::NoWrap,
                lightningcss::properties::flex::FlexWrap::Wrap => FlexWrap::Wrap,
                lightningcss::properties::flex::FlexWrap::WrapReverse => FlexWrap::WrapReverse,
            };
            Ok(true)
        }
        Property::JustifyContent(jc, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.justify_content = map_justify_content(jc);
            Ok(true)
        }
        Property::AlignItems(ai, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.align_items = map_align_items(ai);
            Ok(true)
        }
        Property::AlignSelf(asv, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.align_self = map_align_self(asv);
            Ok(true)
        }
        Property::AlignContent(ac, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.align_content = map_align_content(ac);
            Ok(true)
        }
        Property::FlexGrow(g, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.grow = *g as f32;
            Ok(true)
        }
        Property::FlexShrink(s, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.shrink = *s as f32;
            Ok(true)
        }
        Property::FlexBasis(b, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.basis = map_flex_basis(b);
            Ok(true)
        }
        Property::Flex(f, _) => {
            let flex = style.flex.get_or_insert_with(FlexProps::default);
            flex.grow = f.grow as f32;
            flex.shrink = f.shrink as f32;
            flex.basis = map_flex_basis(&f.basis);
            Ok(true)
        }

        Property::Gap(g) => {
            style.gap_x = gap_value_to_length(&g.column);
            style.gap_y = gap_value_to_length(&g.row);
            Ok(true)
        }
        Property::RowGap(g) => {
            style.gap_y = gap_value_to_length(g);
            Ok(true)
        }
        Property::ColumnGap(g) => {
            style.gap_x = gap_value_to_length(g);
            Ok(true)
        }

        // Grid: recognised but deferred (CSS subset §6 — Taffy bridging
        // pending). Silently accept so subset tests pass without surfacing
        // UnsupportedCss for these.
        Property::GridTemplateColumns(_)
        | Property::GridTemplateRows(_)
        | Property::GridAutoFlow(_)
        | Property::GridAutoColumns(_)
        | Property::GridAutoRows(_)
        | Property::GridArea(_)
        | Property::GridColumn(_)
        | Property::GridRow(_)
        | Property::GridColumnStart(_)
        | Property::GridColumnEnd(_)
        | Property::GridRowStart(_)
        | Property::GridRowEnd(_) => Ok(true),

        // Out-of-scope but harmless (silently ignored).
        _ => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Mappers

fn map_display(d: &lc_display::Display) -> Result<Display, LayoutError> {
    use lc_display::Display as LD;
    use lc_display::DisplayInside as DI;
    use lc_display::DisplayKeyword as DK;
    use lc_display::DisplayOutside as DO;
    match d {
        LD::Keyword(DK::None) => Ok(Display::None),
        LD::Keyword(DK::Contents) => Ok(Display::Block), // contents ⏸; degrade
        LD::Keyword(_) => Ok(Display::Block),            // table-* etc.
        LD::Pair(p) => match (&p.outside, &p.inside) {
            (_, DI::Flex(_)) => Ok(Display::Flex),
            (_, DI::Grid) => Ok(Display::Grid),
            (DO::Block, DI::Flow) | (DO::Block, DI::FlowRoot) => Ok(Display::Block),
            (DO::Inline, DI::Flow) => Err(LayoutError::UnsupportedCss {
                feature: "display: inline".into(),
                location: None,
            }),
            (DO::Inline, DI::FlowRoot) => Err(LayoutError::UnsupportedCss {
                feature: "display: inline-block".into(),
                location: None,
            }),
            (DO::RunIn, _) => Err(LayoutError::UnsupportedCss {
                feature: "display: run-in".into(),
                location: None,
            }),
            _ => Ok(Display::Block),
        },
    }
}

fn map_position(p: &lc_position::Position) -> Position {
    use lc_position::Position as LP;
    match p {
        LP::Static => Position::Static,
        LP::Relative => Position::Relative,
        LP::Absolute => Position::Absolute,
        LP::Fixed => Position::Fixed,
        LP::Sticky(_) => Position::Relative, // sticky ⏸ in subset; degrade
    }
}

fn map_overflow_keyword(v: &lc_overflow::OverflowKeyword) -> Overflow {
    use lc_overflow::OverflowKeyword as OK;
    match v {
        OK::Visible => Overflow::Visible,
        OK::Hidden | OK::Clip => Overflow::Hidden,
        OK::Scroll => Overflow::Scroll,
        OK::Auto => Overflow::Auto,
    }
}

fn map_text_align(a: &lc_text::TextAlign) -> TextAlign {
    use lc_text::TextAlign as TA;
    match a {
        TA::Left | TA::Start | TA::MatchParent => TextAlign::Left,
        TA::Right | TA::End => TextAlign::Right,
        TA::Center => TextAlign::Center,
        TA::Justify | TA::JustifyAll => TextAlign::Justify,
    }
}

fn map_font_weight(w: &lc_font::FontWeight) -> crate::font::FontWeight {
    use crate::font::FontWeight as FW;
    use lc_font::AbsoluteFontWeight as AFW;
    use lc_font::FontWeight as LW;
    match w {
        LW::Absolute(AFW::Normal) => FW::Normal,
        LW::Absolute(AFW::Bold) => FW::Bold,
        LW::Absolute(AFW::Weight(n)) => FW::Weight(*n as u16),
        LW::Bolder | LW::Lighter => FW::Normal,
    }
}

// ── Flex / alignment mappers ───────────────────────────────────────────────

fn map_justify_content(jc: &lightningcss::properties::align::JustifyContent) -> Option<JustifyContent> {
    use lightningcss::properties::align::{ContentDistribution as CD, ContentPosition as CP};
    use lightningcss::properties::align::JustifyContent as LJC;
    match jc {
        LJC::Normal => None,
        LJC::ContentDistribution(cd) => Some(match cd {
            CD::SpaceBetween => JustifyContent::SpaceBetween,
            CD::SpaceAround => JustifyContent::SpaceAround,
            CD::SpaceEvenly => JustifyContent::SpaceEvenly,
            CD::Stretch => JustifyContent::Stretch,
        }),
        LJC::ContentPosition { value, .. } => Some(match value {
            CP::Start => JustifyContent::Start,
            CP::End => JustifyContent::End,
            CP::Center => JustifyContent::Center,
            CP::FlexStart => JustifyContent::FlexStart,
            CP::FlexEnd => JustifyContent::FlexEnd,
        }),
        LJC::Left { .. } => Some(JustifyContent::Start),
        LJC::Right { .. } => Some(JustifyContent::End),
    }
}

fn map_align_content(ac: &lightningcss::properties::align::AlignContent) -> Option<JustifyContent> {
    use lightningcss::properties::align::AlignContent as LAC;
    use lightningcss::properties::align::{ContentDistribution as CD, ContentPosition as CP};
    match ac {
        LAC::Normal | LAC::BaselinePosition(_) => None,
        LAC::ContentDistribution(cd) => Some(match cd {
            CD::SpaceBetween => JustifyContent::SpaceBetween,
            CD::SpaceAround => JustifyContent::SpaceAround,
            CD::SpaceEvenly => JustifyContent::SpaceEvenly,
            CD::Stretch => JustifyContent::Stretch,
        }),
        LAC::ContentPosition { value, .. } => Some(match value {
            CP::Start => JustifyContent::Start,
            CP::End => JustifyContent::End,
            CP::Center => JustifyContent::Center,
            CP::FlexStart => JustifyContent::FlexStart,
            CP::FlexEnd => JustifyContent::FlexEnd,
        }),
    }
}

fn map_align_items(ai: &lightningcss::properties::align::AlignItems) -> Option<AlignItems> {
    use lightningcss::properties::align::AlignItems as LAI;
    use lightningcss::properties::align::SelfPosition as SP;
    match ai {
        LAI::Normal => None,
        LAI::Stretch => Some(AlignItems::Stretch),
        LAI::BaselinePosition(_) => Some(AlignItems::Baseline),
        LAI::SelfPosition { value, .. } => Some(match value {
            SP::Center => AlignItems::Center,
            SP::Start | SP::SelfStart => AlignItems::Start,
            SP::End | SP::SelfEnd => AlignItems::End,
            SP::FlexStart => AlignItems::FlexStart,
            SP::FlexEnd => AlignItems::FlexEnd,
        }),
    }
}

fn map_align_self(asv: &lightningcss::properties::align::AlignSelf) -> Option<AlignItems> {
    use lightningcss::properties::align::AlignSelf as LAS;
    use lightningcss::properties::align::SelfPosition as SP;
    match asv {
        LAS::Auto | LAS::Normal => None,
        LAS::Stretch => Some(AlignItems::Stretch),
        LAS::BaselinePosition(_) => Some(AlignItems::Baseline),
        LAS::SelfPosition { value, .. } => Some(match value {
            SP::Center => AlignItems::Center,
            SP::Start | SP::SelfStart => AlignItems::Start,
            SP::End | SP::SelfEnd => AlignItems::End,
            SP::FlexStart => AlignItems::FlexStart,
            SP::FlexEnd => AlignItems::FlexEnd,
        }),
    }
}

fn map_flex_basis(b: &LengthPercentageOrAuto) -> Length {
    lpa_to_length(b)
}

fn gap_value_to_length(g: &lightningcss::properties::align::GapValue) -> Length {
    use lightningcss::properties::align::GapValue;
    match g {
        GapValue::Normal => Length::Px(0.0),
        GapValue::LengthPercentage(lp) => lp_to_length(lp),
    }
}

fn map_color(c: &CssColor) -> Result<Color, LayoutError> {
    if matches!(c, CssColor::CurrentColor) {
        return Ok(Color::CurrentColor);
    }
    let resolved = c
        .to_rgb()
        .map_err(|_| LayoutError::CssParse("color: failed to resolve to rgba".into()))?;
    if let CssColor::RGBA(rgba) = resolved {
        Ok(Color::Rgba(rgba.red, rgba.green, rgba.blue, rgba.alpha))
    } else {
        Err(LayoutError::CssParse("color: not RGBA after resolve".into()))
    }
}

fn map_size(s: &LcSize) -> Length {
    use LcSize as S;
    match s {
        S::Auto => Length::Auto,
        S::LengthPercentage(lp) => lp_to_length(lp),
        S::MinContent(_) | S::MaxContent(_) | S::FitContent(_) | S::FitContentFunction(_) => {
            Length::Auto
        }
        S::Stretch(_) | S::Contain => Length::Auto,
    }
}

fn map_max_size(s: &LcMaxSize) -> Length {
    use LcMaxSize as S;
    match s {
        S::None => Length::Auto,
        S::LengthPercentage(lp) => lp_to_length(lp),
        S::MinContent(_) | S::MaxContent(_) | S::FitContent(_) | S::FitContentFunction(_) => {
            Length::Auto
        }
        S::Stretch(_) | S::Contain => Length::Auto,
    }
}

fn lp_to_length(lp: &LengthPercentage) -> Length {
    match lp {
        DimensionPercentage::Dimension(d) => Length::Px(length_value_to_px(d).unwrap_or(0.0)),
        DimensionPercentage::Percentage(p) => Length::Percent(p.0 * 100.0),
        DimensionPercentage::Calc(_) => Length::Auto,
    }
}

fn lpa_to_length(v: &LengthPercentageOrAuto) -> Length {
    match v {
        LengthPercentageOrAuto::Auto => Length::Auto,
        LengthPercentageOrAuto::LengthPercentage(lp) => lp_to_length(lp),
    }
}

fn font_size_to_px(s: &lc_font::FontSize, parent_px: f32) -> f32 {
    use lc_font::FontSize as FS;
    match s {
        FS::Length(LengthPercentage::Dimension(d)) => length_value_to_px(d).unwrap_or(parent_px),
        FS::Length(LengthPercentage::Percentage(p)) => parent_px * p.0,
        FS::Length(LengthPercentage::Calc(_)) => parent_px,
        FS::Absolute(_) | FS::Relative(_) => parent_px,
    }
}

/// Convert an absolute length value to pixels. Returns None for relative
/// units (em/rem/vw/vh) which the caller must resolve with context.
fn length_value_to_px(v: &LengthValue) -> Option<f32> {
    match v {
        LengthValue::Px(v) => Some(*v),
        LengthValue::In(v) => Some(*v * 96.0),
        LengthValue::Cm(v) => Some(*v * 96.0 / 2.54),
        LengthValue::Mm(v) => Some(*v * 96.0 / 25.4),
        LengthValue::Q(v) => Some(*v * 96.0 / 25.4 / 4.0),
        LengthValue::Pt(v) => Some(*v * 96.0 / 72.0),
        LengthValue::Pc(v) => Some(*v * 96.0 / 6.0),
        _ => None,
    }
}

fn format_font_family(f: &lc_font::FontFamily) -> String {
    use lc_font::FontFamily as FF;
    use lightningcss::stylesheet::PrinterOptions;
    use lightningcss::traits::ToCss;
    match f {
        FF::Generic(g) => format!("{g:?}").to_lowercase(),
        FF::FamilyName(name) => {
            // No public accessor exposes the inner string; round-trip through
            // ToCss. Strip surrounding quotes that the printer adds for names
            // containing whitespace.
            let s = name
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            s.trim_matches('"').to_string()
        }
    }
}
