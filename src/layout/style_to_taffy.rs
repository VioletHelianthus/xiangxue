//! Convert our [`ComputedStyle`] into a [`taffy::Style`] cached on each Node.
//!
//! Reference: `blitz/packages/stylo_taffy/src/convert.rs` is the field-mapping
//! checklist. Our subset is smaller (no inline-flow, no float, no aspect-ratio
//! shorthand), so this is the trimmed equivalent.

use taffy::style::{
    AlignContent as TafAlignContent, AlignItems as TafAlignItems, AlignSelf as TafAlignSelf,
    Dimension, Display as TafDisplay, FlexDirection as TafFlexDirection,
    FlexWrap as TafFlexWrap, GridAutoFlow as TafGridAutoFlow,
    GridPlacement as TafGridPlacement, GridTemplateArea as TafGridTemplateArea,
    GridTemplateComponent as TafGridTemplateComponent,
    GridTemplateRepetition as TafGridTemplateRepetition, JustifyContent as TafJustifyContent,
    LengthPercentage, LengthPercentageAuto, MaxTrackSizingFunction as TafMaxTrack,
    MinTrackSizingFunction as TafMinTrack, Overflow as TafOverflow, Position as TafPosition,
    RepetitionCount as TafRepetitionCount, Style, TrackSizingFunction as TafTrackSizing,
};
use taffy::geometry::Line as TafLine;
use taffy::Point as TafPoint;
use taffy::Rect as TafRect;
use taffy::Size as TafSize;

use crate::box_model::Sides;
use crate::style::{
    AlignItems, ComputedStyle, Display, FlexDirection, FlexProps, FlexWrap, GridAutoFlow,
    GridLine as XGridLine, GridProps, GridRepeatCount, GridTemplateAreas, GridTemplateComponent,
    GridTrack, GridTrackSize, JustifyContent, Length, Overflow, Position,
};

/// Convert a single `ComputedStyle` into a `taffy::Style`. This is a pure
/// function — input/output only, no document interaction.
pub fn convert(cs: &ComputedStyle) -> Style {
    let mut s = Style::DEFAULT;

    s.display = match cs.display {
        Display::Block => TafDisplay::Block,
        Display::Flex => TafDisplay::Flex,
        Display::Grid => TafDisplay::Grid,
        Display::None => TafDisplay::None,
    };

    s.position = match cs.position {
        Position::Static | Position::Relative => TafPosition::Relative,
        Position::Absolute | Position::Fixed => TafPosition::Absolute,
    };

    s.overflow = TafPoint {
        x: map_overflow(cs.overflow_x),
        y: map_overflow(cs.overflow_y),
    };

    s.size = TafSize {
        width: dim(cs.width),
        height: dim(cs.height),
    };
    s.min_size = TafSize {
        width: dim(cs.min_width),
        height: dim(cs.min_height),
    };
    s.max_size = TafSize {
        width: dim(cs.max_width),
        height: dim(cs.max_height),
    };

    s.padding = sides_to_lp(&cs.padding);
    s.margin = sides_to_lpa(&cs.margin);
    s.inset = sides_to_lpa(&cs.inset);

    if let Some(flex) = &cs.flex {
        apply_flex(&mut s, flex);
    }

    s.gap = TafSize {
        width: length_to_lp(cs.gap_x),
        height: length_to_lp(cs.gap_y),
    };

    if let Some(grid) = &cs.grid {
        apply_grid_container(&mut s, grid);
    }
    apply_grid_placement(&mut s, &cs.grid_column, &cs.grid_row);

    s
}

fn apply_grid_container(s: &mut Style, grid: &GridProps) {
    s.grid_template_columns = grid
        .template_columns
        .iter()
        .map(map_template_component)
        .collect();
    s.grid_template_rows = grid
        .template_rows
        .iter()
        .map(map_template_component)
        .collect();
    s.grid_auto_columns = grid
        .auto_columns
        .iter()
        .map(map_track_sizing)
        .collect();
    s.grid_auto_rows = grid
        .auto_rows
        .iter()
        .map(map_track_sizing)
        .collect();
    s.grid_auto_flow = match grid.auto_flow {
        GridAutoFlow::Row => TafGridAutoFlow::Row,
        GridAutoFlow::Column => TafGridAutoFlow::Column,
        GridAutoFlow::RowDense => TafGridAutoFlow::RowDense,
        GridAutoFlow::ColumnDense => TafGridAutoFlow::ColumnDense,
    };
    if let Some(areas) = &grid.template_areas {
        s.grid_template_areas = expand_template_areas(areas);
    }
}

fn apply_grid_placement(
    s: &mut Style,
    grid_column: &(XGridLine, XGridLine),
    grid_row: &(XGridLine, XGridLine),
) {
    s.grid_column = TafLine {
        start: map_grid_placement(&grid_column.0, true),
        end: map_grid_placement(&grid_column.1, false),
    };
    s.grid_row = TafLine {
        start: map_grid_placement(&grid_row.0, true),
        end: map_grid_placement(&grid_row.1, false),
    };
}

/// Map a single named-area placement to taffy. CSS `grid-area: header` expands
/// to start = `header-start`, end = `header-end` lines per the W3C grid spec
/// (taffy's template-areas implementation generates implicit named lines with
/// these suffixes).
fn map_grid_placement(line: &XGridLine, is_start: bool) -> TafGridPlacement<String> {
    match line {
        XGridLine::Auto => TafGridPlacement::Auto,
        XGridLine::Index(i) => TafGridPlacement::Line((*i).into()),
        XGridLine::Named(name) => {
            let suffix = if is_start { "-start" } else { "-end" };
            TafGridPlacement::NamedLine(format!("{name}{suffix}"), 1)
        }
        XGridLine::Span(n) => TafGridPlacement::Span(*n),
    }
}

fn map_template_component(c: &GridTemplateComponent) -> TafGridTemplateComponent<String> {
    match c {
        GridTemplateComponent::Single(t) => TafGridTemplateComponent::Single(map_track_sizing(t)),
        GridTemplateComponent::Repeat { count, tracks } => {
            TafGridTemplateComponent::Repeat(TafGridTemplateRepetition {
                count: match count {
                    GridRepeatCount::Count(n) => TafRepetitionCount::Count(*n),
                    GridRepeatCount::AutoFill => TafRepetitionCount::AutoFill,
                    GridRepeatCount::AutoFit => TafRepetitionCount::AutoFit,
                },
                tracks: tracks.iter().map(map_track_sizing).collect(),
                line_names: Vec::new(),
            })
        }
    }
}

fn map_track_sizing(t: &GridTrack) -> TafTrackSizing {
    TafTrackSizing {
        min: map_min_track(&t.min),
        max: map_max_track(&t.max),
    }
}

fn map_min_track(s: &GridTrackSize) -> TafMinTrack {
    match s {
        GridTrackSize::Px(v) => TafMinTrack::length(*v),
        GridTrackSize::Percent(p) => TafMinTrack::percent(p / 100.0),
        // Fr is invalid in min position; degrade to Auto (CSS spec).
        GridTrackSize::Fr(_) => TafMinTrack::auto(),
        GridTrackSize::Auto => TafMinTrack::auto(),
        GridTrackSize::MinContent => TafMinTrack::min_content(),
        GridTrackSize::MaxContent => TafMinTrack::max_content(),
    }
}

fn map_max_track(s: &GridTrackSize) -> TafMaxTrack {
    match s {
        GridTrackSize::Px(v) => TafMaxTrack::length(*v),
        GridTrackSize::Percent(p) => TafMaxTrack::percent(p / 100.0),
        GridTrackSize::Fr(v) => TafMaxTrack::fr(*v),
        GridTrackSize::Auto => TafMaxTrack::auto(),
        GridTrackSize::MinContent => TafMaxTrack::min_content(),
        GridTrackSize::MaxContent => TafMaxTrack::max_content(),
    }
}

/// Expand `grid-template-areas` into taffy's `GridTemplateArea` per name. For
/// each named cell, find its bounding box (row/column min..max+1) across the
/// flat row-major area list.
fn expand_template_areas(areas: &GridTemplateAreas) -> Vec<TafGridTemplateArea<String>> {
    let columns = areas.columns.max(1) as usize;
    let row_count = (areas.areas.len() + columns - 1) / columns;
    let mut bounds: std::collections::BTreeMap<String, (u16, u16, u16, u16)> =
        std::collections::BTreeMap::new();
    for r in 0..row_count {
        for c in 0..columns {
            let idx = r * columns + c;
            let cell = match areas.areas.get(idx).and_then(|n| n.as_ref()) {
                Some(n) => n,
                None => continue,
            };
            let r_u = r as u16;
            let c_u = c as u16;
            bounds
                .entry(cell.clone())
                .and_modify(|(rs, re, cs, ce)| {
                    *rs = (*rs).min(r_u);
                    *re = (*re).max(r_u);
                    *cs = (*cs).min(c_u);
                    *ce = (*ce).max(c_u);
                })
                .or_insert((r_u, r_u, c_u, c_u));
        }
    }
    bounds
        .into_iter()
        .map(|(name, (rs, re, cs, ce))| TafGridTemplateArea {
            name,
            row_start: rs + 1,
            row_end: re + 2,
            column_start: cs + 1,
            column_end: ce + 2,
        })
        .collect()
}

fn apply_flex(s: &mut Style, flex: &FlexProps) {
    s.flex_direction = match flex.direction {
        FlexDirection::Row => TafFlexDirection::Row,
        FlexDirection::RowReverse => TafFlexDirection::RowReverse,
        FlexDirection::Column => TafFlexDirection::Column,
        FlexDirection::ColumnReverse => TafFlexDirection::ColumnReverse,
    };
    s.flex_wrap = match flex.wrap {
        FlexWrap::NoWrap => TafFlexWrap::NoWrap,
        FlexWrap::Wrap => TafFlexWrap::Wrap,
        FlexWrap::WrapReverse => TafFlexWrap::WrapReverse,
    };
    s.justify_content = flex.justify_content.map(to_taf_justify);
    s.align_items = flex.align_items.map(to_taf_align_items);
    s.align_self = flex.align_self.map(to_taf_align_items).map(|a| a as TafAlignSelf);
    s.align_content = flex.align_content.map(to_taf_align_content);
    s.flex_grow = flex.grow;
    s.flex_shrink = flex.shrink;
    s.flex_basis = dim(flex.basis);
}

fn to_taf_justify(jc: JustifyContent) -> TafJustifyContent {
    match jc {
        JustifyContent::Start => TafJustifyContent::Start,
        JustifyContent::End => TafJustifyContent::End,
        JustifyContent::FlexStart => TafJustifyContent::FlexStart,
        JustifyContent::FlexEnd => TafJustifyContent::FlexEnd,
        JustifyContent::Center => TafJustifyContent::Center,
        JustifyContent::Stretch => TafJustifyContent::Stretch,
        JustifyContent::SpaceBetween => TafJustifyContent::SpaceBetween,
        JustifyContent::SpaceAround => TafJustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly => TafJustifyContent::SpaceEvenly,
    }
}

fn to_taf_align_items(ai: AlignItems) -> TafAlignItems {
    match ai {
        AlignItems::Start => TafAlignItems::Start,
        AlignItems::End => TafAlignItems::End,
        AlignItems::FlexStart => TafAlignItems::FlexStart,
        AlignItems::FlexEnd => TafAlignItems::FlexEnd,
        AlignItems::Center => TafAlignItems::Center,
        AlignItems::Baseline => TafAlignItems::Baseline,
        AlignItems::Stretch => TafAlignItems::Stretch,
    }
}

fn to_taf_align_content(ac: JustifyContent) -> TafAlignContent {
    // align-content reuses the JustifyContent enum at our layer; mapping is identical.
    to_taf_justify(ac) as TafAlignContent
}

fn map_overflow(o: Overflow) -> TafOverflow {
    match o {
        Overflow::Visible => TafOverflow::Visible,
        Overflow::Hidden => TafOverflow::Hidden,
        Overflow::Scroll => TafOverflow::Scroll,
        // Taffy doesn't have explicit Auto; treat as Scroll.
        Overflow::Auto => TafOverflow::Scroll,
    }
}

fn dim(l: Length) -> Dimension {
    match l {
        Length::Auto => Dimension::auto(),
        Length::Px(v) => Dimension::length(v),
        Length::Percent(p) => Dimension::percent(p / 100.0),
    }
}

fn length_to_lp(l: Length) -> LengthPercentage {
    match l {
        Length::Auto | Length::Px(_) => LengthPercentage::length(match l {
            Length::Px(v) => v,
            _ => 0.0,
        }),
        Length::Percent(p) => LengthPercentage::percent(p / 100.0),
    }
}

fn length_to_lpa(l: Length) -> LengthPercentageAuto {
    match l {
        Length::Auto => LengthPercentageAuto::auto(),
        Length::Px(v) => LengthPercentageAuto::length(v),
        Length::Percent(p) => LengthPercentageAuto::percent(p / 100.0),
    }
}

fn sides_to_lp(s: &Sides<Length>) -> TafRect<LengthPercentage> {
    TafRect {
        top: length_to_lp(s.top),
        right: length_to_lp(s.right),
        bottom: length_to_lp(s.bottom),
        left: length_to_lp(s.left),
    }
}

fn sides_to_lpa(s: &Sides<Length>) -> TafRect<LengthPercentageAuto> {
    TafRect {
        top: length_to_lpa(s.top),
        right: length_to_lpa(s.right),
        bottom: length_to_lpa(s.bottom),
        left: length_to_lpa(s.left),
    }
}
