//! Convert our [`ComputedStyle`] into a [`taffy::Style`] cached on each Node.
//!
//! Reference: `blitz/packages/stylo_taffy/src/convert.rs` is the field-mapping
//! checklist. Our subset is smaller (no inline-flow, no float, no aspect-ratio
//! shorthand), so this is the trimmed equivalent.

use taffy::style::{
    AlignContent as TafAlignContent, AlignItems as TafAlignItems, AlignSelf as TafAlignSelf,
    Dimension, Display as TafDisplay, FlexDirection as TafFlexDirection,
    FlexWrap as TafFlexWrap, JustifyContent as TafJustifyContent, LengthPercentage,
    LengthPercentageAuto, Overflow as TafOverflow, Position as TafPosition, Style,
};
use taffy::Point as TafPoint;
use taffy::Rect as TafRect;
use taffy::Size as TafSize;

use crate::box_model::Sides;
use crate::style::{
    AlignItems, ComputedStyle, Display, FlexDirection, FlexProps, FlexWrap, JustifyContent,
    Length, Overflow, Position,
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

    s
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
