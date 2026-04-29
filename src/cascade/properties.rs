//! Map lightningcss `Property` values to our [`ComputedStyle`].
//!
//! Scope is bounded by the supported CSS subset. Supported properties are
//! mapped here; unsupported ones return `Err(LayoutError::UnsupportedCss)`
//! at the call site (`cascade::mod.rs` decides whether to bail or skip).

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
    AlignItems, Color, ComputedStyle, Display, FlexDirection, FlexProps, FlexWrap, GridAutoFlow,
    GridLine as XGridLine, GridProps, GridRepeatCount, GridTemplateAreas, GridTemplateComponent,
    GridTrack, GridTrackSize, JustifyContent, Length, Overflow, Position, TextAlign, TransformOp,
    Visibility,
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

        // Grid container properties (CSS subset §3.6).
        Property::GridTemplateColumns(value) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.template_columns = map_track_sizing(value)?;
            Ok(true)
        }
        Property::GridTemplateRows(value) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.template_rows = map_track_sizing(value)?;
            Ok(true)
        }
        Property::GridAutoColumns(list) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.auto_columns = map_track_size_list(list)?;
            Ok(true)
        }
        Property::GridAutoRows(list) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.auto_rows = map_track_size_list(list)?;
            Ok(true)
        }
        Property::GridAutoFlow(flow) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.auto_flow = map_grid_auto_flow(flow);
            Ok(true)
        }
        Property::GridTemplateAreas(areas) => {
            let grid = style.grid.get_or_insert_with(GridProps::default);
            grid.template_areas = map_grid_template_areas(areas);
            Ok(true)
        }

        // Grid item placement (per-element).
        Property::GridColumnStart(line) => {
            style.grid_column.0 = map_grid_line(line)?;
            Ok(true)
        }
        Property::GridColumnEnd(line) => {
            style.grid_column.1 = map_grid_line(line)?;
            Ok(true)
        }
        Property::GridRowStart(line) => {
            style.grid_row.0 = map_grid_line(line)?;
            Ok(true)
        }
        Property::GridRowEnd(line) => {
            style.grid_row.1 = map_grid_line(line)?;
            Ok(true)
        }
        Property::GridColumn(shorthand) => {
            style.grid_column.0 = map_grid_line(&shorthand.start)?;
            style.grid_column.1 = map_grid_line(&shorthand.end)?;
            Ok(true)
        }
        Property::GridRow(shorthand) => {
            style.grid_row.0 = map_grid_line(&shorthand.start)?;
            style.grid_row.1 = map_grid_line(&shorthand.end)?;
            Ok(true)
        }
        Property::GridArea(shorthand) => {
            style.grid_row.0 = map_grid_line(&shorthand.row_start)?;
            style.grid_column.0 = map_grid_line(&shorthand.column_start)?;
            style.grid_row.1 = map_grid_line(&shorthand.row_end)?;
            style.grid_column.1 = map_grid_line(&shorthand.column_end)?;
            Ok(true)
        }

        // Transform (CSS subset §3.9).
        Property::Transform(list, _) => {
            style.transforms = map_transform_list(list)?;
            Ok(true)
        }

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
            // Blockify inline-* per CSS Display §9.7 (absolutely positioned / floated boxes).
            // The engine has no inline flow, so inline / inline-block degrade to block,
            // and inline-flex / inline-grid degrade to flex / grid respectively.
            (_, DI::Flex(_)) => Ok(Display::Flex),
            (_, DI::Grid) => Ok(Display::Grid),
            (DO::Block, DI::Flow) | (DO::Block, DI::FlowRoot) => Ok(Display::Block),
            (DO::Inline, DI::Flow) | (DO::Inline, DI::FlowRoot) => Ok(Display::Block),
            (DO::RunIn, _) => Ok(Display::Block),
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

// ---------------------------------------------------------------------------
// Grid mappers (CSS subset §3.6)

fn map_track_sizing(
    value: &lightningcss::properties::grid::TrackSizing<'_>,
) -> Result<Vec<GridTemplateComponent>, LayoutError> {
    use lightningcss::properties::grid::{TrackListItem, TrackSizing};
    match value {
        TrackSizing::None => Ok(Vec::new()),
        TrackSizing::TrackList(list) => {
            let mut out = Vec::with_capacity(list.items.len());
            for item in &list.items {
                match item {
                    TrackListItem::TrackSize(size) => {
                        out.push(GridTemplateComponent::Single(map_track_size(size)?));
                    }
                    TrackListItem::TrackRepeat(rep) => {
                        let mut tracks = Vec::with_capacity(rep.track_sizes.len());
                        for s in &rep.track_sizes {
                            tracks.push(map_track_size(s)?);
                        }
                        out.push(GridTemplateComponent::Repeat {
                            count: map_repeat_count(&rep.count),
                            tracks,
                        });
                    }
                }
            }
            Ok(out)
        }
    }
}

fn map_track_size_list(
    list: &lightningcss::properties::grid::TrackSizeList,
) -> Result<Vec<GridTrack>, LayoutError> {
    let mut out = Vec::with_capacity(list.0.len());
    for s in list.0.iter() {
        out.push(map_track_size(s)?);
    }
    Ok(out)
}

fn map_track_size(
    size: &lightningcss::properties::grid::TrackSize,
) -> Result<GridTrack, LayoutError> {
    use lightningcss::properties::grid::TrackSize;
    match size {
        TrackSize::TrackBreadth(b) => Ok(GridTrack::from_breadth(map_track_breadth(b)?)),
        TrackSize::MinMax { min, max } => Ok(GridTrack {
            min: map_track_breadth(min)?,
            max: map_track_breadth(max)?,
        }),
        TrackSize::FitContent(_) => Err(LayoutError::UnsupportedCss {
            feature: "grid track size: fit-content()".into(),
            location: None,
        }),
    }
}

fn map_track_breadth(
    breadth: &lightningcss::properties::grid::TrackBreadth,
) -> Result<GridTrackSize, LayoutError> {
    use lightningcss::properties::grid::TrackBreadth;
    match breadth {
        TrackBreadth::Length(lp) => Ok(match lp_to_length(lp) {
            Length::Px(v) => GridTrackSize::Px(v),
            Length::Percent(p) => GridTrackSize::Percent(p),
            Length::Auto => GridTrackSize::Auto,
        }),
        TrackBreadth::Flex(v) => Ok(GridTrackSize::Fr(*v as f32)),
        TrackBreadth::MinContent => Ok(GridTrackSize::MinContent),
        TrackBreadth::MaxContent => Ok(GridTrackSize::MaxContent),
        TrackBreadth::Auto => Ok(GridTrackSize::Auto),
    }
}

fn map_repeat_count(
    count: &lightningcss::properties::grid::RepeatCount,
) -> GridRepeatCount {
    use lightningcss::properties::grid::RepeatCount;
    match count {
        RepeatCount::Number(n) => GridRepeatCount::Count((*n).max(0) as u16),
        RepeatCount::AutoFill => GridRepeatCount::AutoFill,
        RepeatCount::AutoFit => GridRepeatCount::AutoFit,
    }
}

fn map_grid_auto_flow(
    flow: &lightningcss::properties::grid::GridAutoFlow,
) -> GridAutoFlow {
    use lightningcss::properties::grid::GridAutoFlow as LcFlow;
    let column = flow.contains(LcFlow::Column);
    let dense = flow.contains(LcFlow::Dense);
    match (column, dense) {
        (false, false) => GridAutoFlow::Row,
        (false, true) => GridAutoFlow::RowDense,
        (true, false) => GridAutoFlow::Column,
        (true, true) => GridAutoFlow::ColumnDense,
    }
}

fn map_grid_template_areas(
    areas: &lightningcss::properties::grid::GridTemplateAreas,
) -> Option<GridTemplateAreas> {
    use lightningcss::properties::grid::GridTemplateAreas as LcAreas;
    match areas {
        LcAreas::None => None,
        LcAreas::Areas { columns, areas } => Some(GridTemplateAreas {
            columns: *columns,
            areas: areas.clone(),
        }),
    }
}

fn map_grid_line(
    line: &lightningcss::properties::grid::GridLine<'_>,
) -> Result<XGridLine, LayoutError> {
    use lightningcss::properties::grid::GridLine as LcGridLine;
    match line {
        LcGridLine::Auto => Ok(XGridLine::Auto),
        LcGridLine::Area { name } => Ok(XGridLine::Named(name.0.as_ref().to_string())),
        LcGridLine::Line { index, name: _ } => Ok(XGridLine::Index(*index as i16)),
        LcGridLine::Span { index, name: None } => {
            Ok(XGridLine::Span((*index).max(1) as u16))
        }
        LcGridLine::Span { name: Some(_), .. } => Err(LayoutError::UnsupportedCss {
            feature: "grid line: span N <name>".into(),
            location: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// Transform mappers (CSS subset §3.9)

fn map_transform_list(
    list: &lightningcss::properties::transform::TransformList,
) -> Result<Vec<TransformOp>, LayoutError> {
    let mut out = Vec::with_capacity(list.0.len());
    for t in &list.0 {
        out.push(map_transform_fn(t)?);
    }
    Ok(out)
}

fn map_transform_fn(
    t: &lightningcss::properties::transform::Transform,
) -> Result<TransformOp, LayoutError> {
    use lightningcss::properties::transform::Transform as T;
    match t {
        T::Translate(x, y) => Ok(TransformOp::Translate {
            x: lp_to_length(x),
            y: lp_to_length(y),
        }),
        T::TranslateX(x) => Ok(TransformOp::Translate {
            x: lp_to_length(x),
            y: Length::Px(0.0),
        }),
        T::TranslateY(y) => Ok(TransformOp::Translate {
            x: Length::Px(0.0),
            y: lp_to_length(y),
        }),
        T::Rotate(a) | T::RotateZ(a) => Ok(TransformOp::Rotate(a.to_degrees())),
        T::Scale(x, y) => Ok(TransformOp::Scale(
            number_or_percent(x),
            number_or_percent(y),
        )),
        T::ScaleX(x) => Ok(TransformOp::Scale(number_or_percent(x), 1.0)),
        T::ScaleY(y) => Ok(TransformOp::Scale(1.0, number_or_percent(y))),
        // 3D / matrix / skew / perspective: outside CSS subset §3.9.
        T::TranslateZ(_) | T::Translate3d(..) => Err(LayoutError::UnsupportedCss {
            feature: "transform: 3D translate".into(),
            location: None,
        }),
        T::ScaleZ(_) | T::Scale3d(..) => Err(LayoutError::UnsupportedCss {
            feature: "transform: 3D scale".into(),
            location: None,
        }),
        T::RotateX(_) | T::RotateY(_) | T::Rotate3d(..) => Err(LayoutError::UnsupportedCss {
            feature: "transform: 3D rotate".into(),
            location: None,
        }),
        T::Skew(..) | T::SkewX(_) | T::SkewY(_) => Err(LayoutError::UnsupportedCss {
            feature: "transform: skew()".into(),
            location: None,
        }),
        T::Matrix(_) | T::Matrix3d(_) => Err(LayoutError::UnsupportedCss {
            feature: "transform: matrix()".into(),
            location: None,
        }),
        T::Perspective(_) => Err(LayoutError::UnsupportedCss {
            feature: "transform: perspective()".into(),
            location: None,
        }),
    }
}

fn number_or_percent(v: &lightningcss::values::percentage::NumberOrPercentage) -> f32 {
    use lightningcss::values::percentage::NumberOrPercentage as N;
    match v {
        N::Number(n) => *n as f32,
        N::Percentage(p) => p.0 as f32,
    }
}
