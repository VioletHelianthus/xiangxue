use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, Token,
};
use cssparser_color::Color;

use crate::types::{Dimension, LayoutProps, Orientation};

/// Parse all recognized layout properties from an inline style string.
pub fn parse_layout_props(style: &str) -> LayoutProps {
    let mut props = LayoutProps::default();
    let mut input = ParserInput::new(style);
    let mut parser = Parser::new(&mut input);
    let mut decl_parser = LayoutDeclParser;
    let iter = RuleBodyParser::new(&mut parser, &mut decl_parser);
    for result in iter {
        if let Ok(decl) = result {
            apply_declaration(&mut props, decl);
        }
        // Silently ignore parse errors for unknown/unsupported properties.
    }
    props
}

/// Backward-compatible wrapper.
pub fn parse_flex_direction(style: &str) -> Option<Orientation> {
    parse_layout_props(style).flex_direction
}

/// Backward-compatible wrapper.
pub fn has_overflow_scroll(style: &str) -> bool {
    parse_layout_props(style).overflow_scroll
}

// ── Internal types ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum TransformFn {
    Scale(f32, f32),
    Rotate(f32), // degrees
}

enum Decl {
    Width(Dimension),
    Height(Dimension),
    Left(Dimension),
    Top(Dimension),
    Right(Dimension),
    Bottom(Dimension),
    MarginShorthand(Vec<f32>),
    MarginTop(f32),
    MarginRight(f32),
    MarginBottom(f32),
    MarginLeft(f32),
    PaddingShorthand(Vec<f32>),
    PaddingTop(f32),
    PaddingRight(f32),
    PaddingBottom(f32),
    PaddingLeft(f32),
    Gap(f32),
    TransformOrigin(Option<f32>, Option<f32>),
    FlexDirection(Orientation),
    OverflowScroll,
    Display(taffy::Display),
    Position(taffy::Position),
    JustifyContent(taffy::JustifyContent),
    AlignItems(taffy::AlignItems),
    AlignSelf(taffy::AlignSelf),
    FlexWrap(taffy::FlexWrap),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(Dimension),
    BackgroundImage(String),
    Transform(Vec<TransformFn>),
    Opacity(f32),
    Visibility(bool),
    ZIndex(i32),
    Color(u8, u8, u8),
    BackgroundColor(u8, u8, u8, u8),
}

fn apply_declaration(props: &mut LayoutProps, decl: Decl) {
    match decl {
        Decl::Width(d) => props.width = Some(d),
        Decl::Height(d) => props.height = Some(d),
        Decl::Left(d) => props.left = Some(d),
        Decl::Top(d) => props.top = Some(d),
        Decl::MarginTop(v) => props.margin_top = Some(v),
        Decl::MarginRight(v) => props.margin_right = Some(v),
        Decl::MarginBottom(v) => props.margin_bottom = Some(v),
        Decl::MarginLeft(v) => props.margin_left = Some(v),
        Decl::MarginShorthand(vals) => apply_box_shorthand_margin(props, &vals),
        Decl::PaddingTop(v) => props.padding_top = Some(v),
        Decl::PaddingRight(v) => props.padding_right = Some(v),
        Decl::PaddingBottom(v) => props.padding_bottom = Some(v),
        Decl::PaddingLeft(v) => props.padding_left = Some(v),
        Decl::PaddingShorthand(vals) => apply_box_shorthand_padding(props, &vals),
        Decl::Gap(v) => props.gap = Some(v),
        Decl::TransformOrigin(x, y) => {
            if let Some(x) = x {
                props.anchor_x = Some(x);
            }
            if let Some(y) = y {
                props.anchor_y = Some(y);
            }
        }
        Decl::FlexDirection(o) => props.flex_direction = Some(o),
        Decl::OverflowScroll => props.overflow_scroll = true,
        Decl::Right(d) => props.right = Some(d),
        Decl::Bottom(d) => props.bottom = Some(d),
        Decl::Display(d) => props.display = Some(d),
        Decl::Position(p) => props.position = Some(p),
        Decl::JustifyContent(jc) => props.justify_content = Some(jc),
        Decl::AlignItems(ai) => props.align_items = Some(ai),
        Decl::AlignSelf(a) => props.align_self = Some(a),
        Decl::FlexWrap(fw) => props.flex_wrap = Some(fw),
        Decl::FlexGrow(v) => props.flex_grow = Some(v),
        Decl::FlexShrink(v) => props.flex_shrink = Some(v),
        Decl::FlexBasis(d) => props.flex_basis = Some(d),
        Decl::BackgroundImage(url) => props.background_image = Some(url),
        Decl::Transform(fns) => {
            props.scale_x = None;
            props.scale_y = None;
            props.rotation = None;
            for tf in fns {
                match tf {
                    TransformFn::Scale(x, y) => {
                        props.scale_x = Some(x);
                        props.scale_y = Some(y);
                    }
                    TransformFn::Rotate(deg) => {
                        props.rotation = Some(deg);
                    }
                }
            }
        }
        Decl::Opacity(v) => props.opacity = Some(v),
        Decl::Visibility(v) => props.visible = Some(v),
        Decl::ZIndex(v) => props.z_order = Some(v),
        Decl::Color(r, g, b) => props.color = Some((r, g, b)),
        Decl::BackgroundColor(r, g, b, a) => props.background_color = Some((r, g, b, a)),
    }
}

fn apply_box_shorthand_margin(props: &mut LayoutProps, vals: &[f32]) {
    match vals.len() {
        1 => {
            let v = vals[0];
            props.margin_top = Some(v);
            props.margin_right = Some(v);
            props.margin_bottom = Some(v);
            props.margin_left = Some(v);
        }
        2 => {
            props.margin_top = Some(vals[0]);
            props.margin_bottom = Some(vals[0]);
            props.margin_right = Some(vals[1]);
            props.margin_left = Some(vals[1]);
        }
        3 => {
            props.margin_top = Some(vals[0]);
            props.margin_right = Some(vals[1]);
            props.margin_left = Some(vals[1]);
            props.margin_bottom = Some(vals[2]);
        }
        4 => {
            props.margin_top = Some(vals[0]);
            props.margin_right = Some(vals[1]);
            props.margin_bottom = Some(vals[2]);
            props.margin_left = Some(vals[3]);
        }
        _ => {}
    }
}

fn apply_box_shorthand_padding(props: &mut LayoutProps, vals: &[f32]) {
    match vals.len() {
        1 => {
            let v = vals[0];
            props.padding_top = Some(v);
            props.padding_right = Some(v);
            props.padding_bottom = Some(v);
            props.padding_left = Some(v);
        }
        2 => {
            props.padding_top = Some(vals[0]);
            props.padding_bottom = Some(vals[0]);
            props.padding_right = Some(vals[1]);
            props.padding_left = Some(vals[1]);
        }
        3 => {
            props.padding_top = Some(vals[0]);
            props.padding_right = Some(vals[1]);
            props.padding_left = Some(vals[1]);
            props.padding_bottom = Some(vals[2]);
        }
        4 => {
            props.padding_top = Some(vals[0]);
            props.padding_right = Some(vals[1]);
            props.padding_bottom = Some(vals[2]);
            props.padding_left = Some(vals[3]);
        }
        _ => {}
    }
}

// ── cssparser DeclarationParser ─────────────────────────────────────────

struct LayoutDeclParser;

impl<'i> DeclarationParser<'i> for LayoutDeclParser {
    type Declaration = Decl;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _start: &ParserState,
    ) -> Result<Decl, ParseError<'i, ()>> {
        match &*name {
            "width" => parse_dimension(input).map(Decl::Width),
            "height" => parse_dimension(input).map(Decl::Height),
            "left" => parse_dimension(input).map(Decl::Left),
            "top" => parse_dimension(input).map(Decl::Top),
            "margin" => parse_px_shorthand(input).map(Decl::MarginShorthand),
            "margin-top" => parse_px_value(input).map(Decl::MarginTop),
            "margin-right" => parse_px_value(input).map(Decl::MarginRight),
            "margin-bottom" => parse_px_value(input).map(Decl::MarginBottom),
            "margin-left" => parse_px_value(input).map(Decl::MarginLeft),
            "padding" => parse_px_shorthand(input).map(Decl::PaddingShorthand),
            "padding-top" => parse_px_value(input).map(Decl::PaddingTop),
            "padding-right" => parse_px_value(input).map(Decl::PaddingRight),
            "padding-bottom" => parse_px_value(input).map(Decl::PaddingBottom),
            "padding-left" => parse_px_value(input).map(Decl::PaddingLeft),
            "gap" => parse_px_value(input).map(Decl::Gap),
            "transform-origin" => parse_transform_origin(input),
            "right" => parse_dimension(input).map(Decl::Right),
            "bottom" => parse_dimension(input).map(Decl::Bottom),
            "flex-direction" => parse_flex_dir(input),
            "overflow" | "overflow-x" | "overflow-y" => parse_overflow(input),
            "display" => parse_taffy_keyword::<taffy::Display>(input).map(Decl::Display),
            "position" => parse_taffy_keyword::<taffy::Position>(input).map(Decl::Position),
            "justify-content" => parse_taffy_keyword::<taffy::JustifyContent>(input).map(Decl::JustifyContent),
            "align-items" => parse_taffy_keyword::<taffy::AlignItems>(input).map(Decl::AlignItems),
            "align-self" => parse_taffy_keyword::<taffy::AlignSelf>(input).map(Decl::AlignSelf),
            "flex-wrap" => parse_taffy_keyword::<taffy::FlexWrap>(input).map(Decl::FlexWrap),
            "flex-grow" => parse_number(input).map(Decl::FlexGrow),
            "flex-shrink" => parse_number(input).map(Decl::FlexShrink),
            "flex-basis" => parse_dimension(input).map(Decl::FlexBasis),
            "background-image" | "background" => parse_background_image(input),
            "transform" => parse_transform(input),
            "opacity" => parse_number(input).map(Decl::Opacity),
            "visibility" => parse_visibility(input),
            "z-index" => parse_integer(input).map(Decl::ZIndex),
            "color" => parse_css_color(input),
            "background-color" => parse_css_background_color(input),
            _ => Err(input.new_custom_error(())),
        }
    }
}

impl<'i> AtRuleParser<'i> for LayoutDeclParser {
    type Prelude = ();
    type AtRule = Decl;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for LayoutDeclParser {
    type Prelude = ();
    type QualifiedRule = Decl;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Decl, ()> for LayoutDeclParser {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        false
    }
}

// ── Value parsers ───────────────────────────────────────────────────────

fn parse_dimension<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Dimension, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match &token {
        Token::Dimension { value, unit, .. } => {
            if unit.eq_ignore_ascii_case("px") {
                Ok(Dimension::Px(*value))
            } else {
                Err(input.new_custom_error(()))
            }
        }
        Token::Percentage { unit_value, .. } => Ok(Dimension::Percent(*unit_value * 100.0)),
        Token::Number { value, .. } if *value == 0.0 => Ok(Dimension::Px(0.0)),
        _ => Err(input.new_custom_error(())),
    }
}

/// Parse `background-image: url('path')` or `background: url('path')`.
/// Handles `url('...')`, `url("...")`, and `url(...)` forms.
fn parse_background_image<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Decl, ParseError<'i, ()>> {
    // cssparser has special handling: url(...) is either a Token::UnquotedUrl
    // or a Token::Function("url") depending on quoting.
    let token = input.next()?.clone();
    match &token {
        Token::UnquotedUrl(url) => Ok(Decl::BackgroundImage(url.to_string())),
        Token::Function(name) if name.eq_ignore_ascii_case("url") => {
            let url = input.parse_nested_block(|input| {
                let t = input.next()?.clone();
                match &t {
                    Token::QuotedString(s) => Ok(s.to_string()),
                    _ => Err(input.new_custom_error(())),
                }
            })?;
            Ok(Decl::BackgroundImage(url))
        }
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_px_value<'i, 't>(input: &mut Parser<'i, 't>) -> Result<f32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match &token {
        Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("px") => Ok(*value),
        Token::Number { value, .. } if *value == 0.0 => Ok(0.0),
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_px_shorthand<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Vec<f32>, ParseError<'i, ()>> {
    let mut values = Vec::new();
    // Parse first value (required)
    values.push(parse_px_value(input)?);
    // Parse remaining values (optional, up to 3 more)
    for _ in 0..3 {
        if input.is_exhausted() {
            break;
        }
        match parse_px_value(input) {
            Ok(v) => values.push(v),
            Err(_) => break,
        }
    }
    Ok(values)
}

/// Parse a bare number (for flex-grow, flex-shrink).
fn parse_number<'i, 't>(input: &mut Parser<'i, 't>) -> Result<f32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match &token {
        Token::Number { value, .. } => Ok(*value),
        _ => Err(input.new_custom_error(())),
    }
}

/// Parse a CSS keyword value using Taffy's FromStr implementation.
fn parse_taffy_keyword<'i, 't, T: std::str::FromStr>(
    input: &mut Parser<'i, 't>,
) -> Result<T, ParseError<'i, ()>> {
    let ident = input.expect_ident()?.clone();
    ident.parse::<T>().map_err(|_| input.new_custom_error(()))
}

fn parse_transform_origin<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Decl, ParseError<'i, ()>> {
    // Collect all component values
    let mut components: Vec<OriginComponent> = Vec::new();
    while !input.is_exhausted() {
        if let Ok(c) = parse_origin_component(input) {
            components.push(c);
        } else {
            break;
        }
    }

    if components.is_empty() {
        return Err(input.new_custom_error(()));
    }

    let (x, y) = resolve_origin_components(&components);
    Ok(Decl::TransformOrigin(x, y))
}

#[derive(Debug, Clone)]
enum OriginComponent {
    Keyword(OriginKeyword),
    Percent(f32), // 0-1
    Px,           // px values are ignored
}

#[derive(Debug, Clone, Copy)]
enum OriginKeyword {
    Left,
    Center,
    Right,
    Top,
    Bottom,
}

fn parse_origin_component<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<OriginComponent, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match &token {
        Token::Ident(ident) => {
            let kw = match &**ident {
                "left" => OriginKeyword::Left,
                "center" => OriginKeyword::Center,
                "right" => OriginKeyword::Right,
                "top" => OriginKeyword::Top,
                "bottom" => OriginKeyword::Bottom,
                _ => return Err(input.new_custom_error(())),
            };
            Ok(OriginComponent::Keyword(kw))
        }
        Token::Percentage { unit_value, .. } => Ok(OriginComponent::Percent(*unit_value)),
        Token::Dimension { unit, .. } if unit.eq_ignore_ascii_case("px") => {
            Ok(OriginComponent::Px)
        }
        Token::Number { value, .. } if *value == 0.0 => Ok(OriginComponent::Percent(0.0)),
        _ => Err(input.new_custom_error(())),
    }
}

fn is_x_keyword(kw: OriginKeyword) -> bool {
    matches!(kw, OriginKeyword::Left | OriginKeyword::Right)
}

fn is_y_keyword(kw: OriginKeyword) -> bool {
    matches!(kw, OriginKeyword::Top | OriginKeyword::Bottom)
}

fn keyword_to_value(kw: OriginKeyword) -> f32 {
    match kw {
        OriginKeyword::Left | OriginKeyword::Top => 0.0,
        OriginKeyword::Center => 0.5,
        OriginKeyword::Right | OriginKeyword::Bottom => 1.0,
    }
}

fn resolve_origin_components(components: &[OriginComponent]) -> (Option<f32>, Option<f32>) {
    match components.len() {
        1 => {
            // Single value: sets X, Y defaults to center (0.5)
            match &components[0] {
                OriginComponent::Keyword(kw) => {
                    if is_y_keyword(*kw) {
                        // "top" or "bottom" as single value → x=center, y=keyword
                        (Some(0.5), Some(keyword_to_value(*kw)))
                    } else {
                        // "left", "right", "center" → x=keyword, y=center
                        (Some(keyword_to_value(*kw)), Some(0.5))
                    }
                }
                OriginComponent::Percent(v) => (Some(*v), Some(0.5)),
                OriginComponent::Px => (None, None),
            }
        }
        2 => resolve_two_components(&components[0], &components[1]),
        _ => {
            // 3+ values: take first two, ignore z
            resolve_two_components(&components[0], &components[1])
        }
    }
}

fn resolve_two_components(
    first: &OriginComponent,
    second: &OriginComponent,
) -> (Option<f32>, Option<f32>) {
    match (first, second) {
        (OriginComponent::Keyword(a), OriginComponent::Keyword(b)) => {
            // If both are axis-specific, assign accordingly
            if is_x_keyword(*a) && is_y_keyword(*b) {
                (Some(keyword_to_value(*a)), Some(keyword_to_value(*b)))
            } else if is_y_keyword(*a) && is_x_keyword(*b) {
                (Some(keyword_to_value(*b)), Some(keyword_to_value(*a)))
            } else if matches!(a, OriginKeyword::Center) && is_y_keyword(*b) {
                (Some(0.5), Some(keyword_to_value(*b)))
            } else if is_x_keyword(*a) && matches!(b, OriginKeyword::Center) {
                (Some(keyword_to_value(*a)), Some(0.5))
            } else if matches!(a, OriginKeyword::Center) && matches!(b, OriginKeyword::Center) {
                (Some(0.5), Some(0.5))
            } else if is_y_keyword(*a) && matches!(b, OriginKeyword::Center) {
                (Some(0.5), Some(keyword_to_value(*a)))
            } else if matches!(a, OriginKeyword::Center) && is_x_keyword(*b) {
                (Some(keyword_to_value(*b)), Some(0.5))
            } else {
                // Fallback: first=x, second=y
                (Some(keyword_to_value(*a)), Some(keyword_to_value(*b)))
            }
        }
        (OriginComponent::Keyword(a), OriginComponent::Percent(pct)) => {
            if is_y_keyword(*a) {
                // e.g. "top 25%" → ambiguous, treat first=y, second=x
                (Some(*pct), Some(keyword_to_value(*a)))
            } else {
                (Some(keyword_to_value(*a)), Some(*pct))
            }
        }
        (OriginComponent::Percent(pct), OriginComponent::Keyword(b)) => {
            if is_y_keyword(*b) {
                (Some(*pct), Some(keyword_to_value(*b)))
            } else if is_x_keyword(*b) {
                (Some(keyword_to_value(*b)), Some(*pct))
            } else {
                (Some(*pct), Some(0.5))
            }
        }
        (OriginComponent::Percent(x), OriginComponent::Percent(y)) => (Some(*x), Some(*y)),
        // Any px involvement → None for that axis
        (OriginComponent::Px, OriginComponent::Px) => (None, None),
        (OriginComponent::Px, _) | (_, OriginComponent::Px) => (None, None),
    }
}

fn parse_flex_dir<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Decl, ParseError<'i, ()>> {
    let ident = input.expect_ident()?.clone();
    match &*ident {
        "row" | "row-reverse" => Ok(Decl::FlexDirection(Orientation::Horizontal)),
        "column" | "column-reverse" => Ok(Decl::FlexDirection(Orientation::Vertical)),
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_overflow<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Decl, ParseError<'i, ()>> {
    let ident = input.expect_ident()?.clone();
    match &*ident {
        "scroll" | "auto" => Ok(Decl::OverflowScroll),
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_transform<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Decl, ParseError<'i, ()>> {
    let mut fns = Vec::new();
    while !input.is_exhausted() {
        let Ok(name) = input.expect_function().cloned() else {
            break;
        };
        let name_lower = name.to_ascii_lowercase();
        let tf = input.parse_nested_block(|input| match name_lower.as_ref() {
            "scale" => {
                let x = parse_number(input)?;
                let y = if !input.is_exhausted() {
                    let _ = input.try_parse(|i| i.expect_comma());
                    parse_number(input).unwrap_or(x)
                } else {
                    x
                };
                Ok(TransformFn::Scale(x, y))
            }
            "scalex" => Ok(TransformFn::Scale(parse_number(input)?, 1.0)),
            "scaley" => Ok(TransformFn::Scale(1.0, parse_number(input)?)),
            "rotate" => {
                let token = input.next()?.clone();
                match &token {
                    Token::Dimension { value, unit, .. } => {
                        let deg = if unit.eq_ignore_ascii_case("deg") {
                            *value
                        } else if unit.eq_ignore_ascii_case("rad") {
                            value.to_degrees()
                        } else if unit.eq_ignore_ascii_case("turn") {
                            *value * 360.0
                        } else {
                            return Err(input.new_custom_error(()));
                        };
                        Ok(TransformFn::Rotate(deg))
                    }
                    Token::Number { value, .. } if *value == 0.0 => Ok(TransformFn::Rotate(0.0)),
                    _ => Err(input.new_custom_error(())),
                }
            }
            _ => {
                while input.next().is_ok() {}
                Err(input.new_custom_error(()))
            }
        });
        if let Ok(tf) = tf {
            fns.push(tf);
        }
    }
    if fns.is_empty() {
        Err(input.new_custom_error(()))
    } else {
        Ok(Decl::Transform(fns))
    }
}

fn parse_visibility<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Decl, ParseError<'i, ()>> {
    let ident = input.expect_ident()?.clone();
    match &*ident {
        "visible" => Ok(Decl::Visibility(true)),
        "hidden" | "collapse" => Ok(Decl::Visibility(false)),
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_integer<'i, 't>(input: &mut Parser<'i, 't>) -> Result<i32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match &token {
        Token::Number {
            int_value: Some(v), ..
        } => Ok(*v),
        Token::Number { value, .. } => Ok(*value as i32),
        _ => Err(input.new_custom_error(())),
    }
}

fn color_to_rgba(color: &Color) -> (u8, u8, u8, u8) {
    match color {
        Color::Rgba(rgba) => (rgba.red, rgba.green, rgba.blue, (rgba.alpha * 255.0).round() as u8),
        Color::CurrentColor => (255, 255, 255, 255),
        // For HSL/HWB/Lab/etc., convert via the CSS serialization path isn't trivial.
        // For game UI, these color spaces are rare. Fall back to white.
        _ => (255, 255, 255, 255),
    }
}

fn parse_css_color<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Decl, ParseError<'i, ()>> {
    let color = Color::parse(input).map_err(|_| input.new_custom_error(()))?;
    let (r, g, b, _) = color_to_rgba(&color);
    Ok(Decl::Color(r, g, b))
}

fn parse_css_background_color<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Decl, ParseError<'i, ()>> {
    let color = Color::parse(input).map_err(|_| input.new_custom_error(()))?;
    let (r, g, b, a) = color_to_rgba(&color);
    Ok(Decl::BackgroundColor(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Dimension;

    // ── Existing backward-compat tests ──────────────────────────────────

    #[test]
    fn flex_direction_row() {
        assert_eq!(
            parse_flex_direction("display: flex; flex-direction: row"),
            Some(Orientation::Horizontal)
        );
    }

    #[test]
    fn flex_direction_column() {
        assert_eq!(
            parse_flex_direction("flex-direction: column"),
            Some(Orientation::Vertical)
        );
    }

    #[test]
    fn flex_direction_absent() {
        assert_eq!(parse_flex_direction("display: flex; color: red"), None);
    }

    #[test]
    fn overflow_scroll() {
        assert!(has_overflow_scroll("overflow: scroll"));
    }

    #[test]
    fn overflow_auto() {
        assert!(has_overflow_scroll("overflow: auto"));
    }

    #[test]
    fn overflow_hidden() {
        assert!(!has_overflow_scroll("overflow: hidden"));
    }

    #[test]
    fn overflow_y_scroll() {
        assert!(has_overflow_scroll("overflow-y: scroll"));
    }

    #[test]
    fn no_overflow() {
        assert!(!has_overflow_scroll("color: red"));
    }

    // ── New layout property tests ───────────────────────────────────────

    #[test]
    fn width_height_px() {
        let p = parse_layout_props("width: 200px; height: 60px");
        assert_eq!(p.width, Some(Dimension::Px(200.0)));
        assert_eq!(p.height, Some(Dimension::Px(60.0)));
    }

    #[test]
    fn width_height_percent() {
        let p = parse_layout_props("width: 50%; height: 100%");
        assert_eq!(p.width, Some(Dimension::Percent(50.0)));
        assert_eq!(p.height, Some(Dimension::Percent(100.0)));
    }

    #[test]
    fn left_top_combination() {
        let p = parse_layout_props("left: 220px; top: 450px");
        assert_eq!(p.left, Some(Dimension::Px(220.0)));
        assert_eq!(p.top, Some(Dimension::Px(450.0)));
    }

    #[test]
    fn left_top_percent() {
        let p = parse_layout_props("left: 25%; top: 50%");
        assert_eq!(p.left, Some(Dimension::Percent(25.0)));
        assert_eq!(p.top, Some(Dimension::Percent(50.0)));
    }

    #[test]
    fn margin_shorthand_one_value() {
        let p = parse_layout_props("margin: 10px");
        assert_eq!(p.margin_top, Some(10.0));
        assert_eq!(p.margin_right, Some(10.0));
        assert_eq!(p.margin_bottom, Some(10.0));
        assert_eq!(p.margin_left, Some(10.0));
    }

    #[test]
    fn margin_shorthand_two_values() {
        let p = parse_layout_props("margin: 10px 20px");
        assert_eq!(p.margin_top, Some(10.0));
        assert_eq!(p.margin_bottom, Some(10.0));
        assert_eq!(p.margin_right, Some(20.0));
        assert_eq!(p.margin_left, Some(20.0));
    }

    #[test]
    fn margin_shorthand_three_values() {
        let p = parse_layout_props("margin: 10px 20px 30px");
        assert_eq!(p.margin_top, Some(10.0));
        assert_eq!(p.margin_right, Some(20.0));
        assert_eq!(p.margin_left, Some(20.0));
        assert_eq!(p.margin_bottom, Some(30.0));
    }

    #[test]
    fn margin_shorthand_four_values() {
        let p = parse_layout_props("margin: 10px 20px 30px 40px");
        assert_eq!(p.margin_top, Some(10.0));
        assert_eq!(p.margin_right, Some(20.0));
        assert_eq!(p.margin_bottom, Some(30.0));
        assert_eq!(p.margin_left, Some(40.0));
    }

    #[test]
    fn margin_individual_properties() {
        let p = parse_layout_props("margin-top: 5px; margin-left: 15px");
        assert_eq!(p.margin_top, Some(5.0));
        assert_eq!(p.margin_left, Some(15.0));
        assert_eq!(p.margin_bottom, None);
        assert_eq!(p.margin_right, None);
    }

    #[test]
    fn padding_shorthand() {
        let p = parse_layout_props("padding: 8px 16px");
        assert_eq!(p.padding_top, Some(8.0));
        assert_eq!(p.padding_bottom, Some(8.0));
        assert_eq!(p.padding_right, Some(16.0));
        assert_eq!(p.padding_left, Some(16.0));
    }

    #[test]
    fn gap_value() {
        let p = parse_layout_props("gap: 12px");
        assert_eq!(p.gap, Some(12.0));
    }

    #[test]
    fn transform_origin_center() {
        let p = parse_layout_props("transform-origin: center");
        assert_eq!(p.anchor_x, Some(0.5));
        assert_eq!(p.anchor_y, Some(0.5));
    }

    #[test]
    fn transform_origin_left_top() {
        let p = parse_layout_props("transform-origin: left top");
        assert_eq!(p.anchor_x, Some(0.0));
        assert_eq!(p.anchor_y, Some(0.0));
    }

    #[test]
    fn transform_origin_right_bottom() {
        let p = parse_layout_props("transform-origin: right bottom");
        assert_eq!(p.anchor_x, Some(1.0));
        assert_eq!(p.anchor_y, Some(1.0));
    }

    #[test]
    fn transform_origin_percentages() {
        let p = parse_layout_props("transform-origin: 25% 75%");
        assert_eq!(p.anchor_x, Some(0.25));
        assert_eq!(p.anchor_y, Some(0.75));
    }

    #[test]
    fn combined_style_string() {
        let p = parse_layout_props(
            "width: 200px; height: 60px; left: 100px; top: 50px; transform-origin: center; gap: 5px",
        );
        assert_eq!(p.width, Some(Dimension::Px(200.0)));
        assert_eq!(p.height, Some(Dimension::Px(60.0)));
        assert_eq!(p.left, Some(Dimension::Px(100.0)));
        assert_eq!(p.top, Some(Dimension::Px(50.0)));
        assert_eq!(p.anchor_x, Some(0.5));
        assert_eq!(p.anchor_y, Some(0.5));
        assert_eq!(p.gap, Some(5.0));
    }

    #[test]
    fn unknown_properties_ignored() {
        let p = parse_layout_props("color: red; background: blue; width: 100px");
        assert_eq!(p.width, Some(Dimension::Px(100.0)));
        // Unknown props don't cause errors
        assert_eq!(p.height, None);
    }

    // ── New Taffy layout property tests ──────────────────────────────────

    #[test]
    fn display_flex() {
        let p = parse_layout_props("display: flex");
        assert_eq!(p.display, Some(taffy::Display::Flex));
    }

    #[test]
    fn display_none() {
        let p = parse_layout_props("display: none");
        assert_eq!(p.display, Some(taffy::Display::None));
    }

    #[test]
    fn position_absolute() {
        let p = parse_layout_props("position: absolute");
        assert_eq!(p.position, Some(taffy::Position::Absolute));
    }

    #[test]
    fn position_relative() {
        let p = parse_layout_props("position: relative");
        assert_eq!(p.position, Some(taffy::Position::Relative));
    }

    #[test]
    fn justify_content_values() {
        let p = parse_layout_props("justify-content: center");
        assert_eq!(p.justify_content, Some(taffy::AlignContent::Center));

        let p = parse_layout_props("justify-content: space-between");
        assert_eq!(p.justify_content, Some(taffy::AlignContent::SpaceBetween));

        let p = parse_layout_props("justify-content: flex-end");
        assert_eq!(p.justify_content, Some(taffy::AlignContent::FlexEnd));
    }

    #[test]
    fn align_items_values() {
        let p = parse_layout_props("align-items: center");
        assert_eq!(p.align_items, Some(taffy::AlignItems::Center));

        let p = parse_layout_props("align-items: stretch");
        assert_eq!(p.align_items, Some(taffy::AlignItems::Stretch));

        let p = parse_layout_props("align-items: flex-start");
        assert_eq!(p.align_items, Some(taffy::AlignItems::FlexStart));
    }

    #[test]
    fn flex_wrap_values() {
        let p = parse_layout_props("flex-wrap: wrap");
        assert_eq!(p.flex_wrap, Some(taffy::FlexWrap::Wrap));

        let p = parse_layout_props("flex-wrap: nowrap");
        assert_eq!(p.flex_wrap, Some(taffy::FlexWrap::NoWrap));
    }

    #[test]
    fn flex_grow_shrink() {
        let p = parse_layout_props("flex-grow: 2; flex-shrink: 0");
        assert_eq!(p.flex_grow, Some(2.0));
        assert_eq!(p.flex_shrink, Some(0.0));
    }

    #[test]
    fn flex_basis_px() {
        let p = parse_layout_props("flex-basis: 100px");
        assert_eq!(p.flex_basis, Some(Dimension::Px(100.0)));
    }

    #[test]
    fn right_bottom() {
        let p = parse_layout_props("right: 10px; bottom: 20px");
        assert_eq!(p.right, Some(Dimension::Px(10.0)));
        assert_eq!(p.bottom, Some(Dimension::Px(20.0)));
    }

    #[test]
    fn combined_flex_layout() {
        let p = parse_layout_props(
            "display:flex;flex-direction:row;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:10px",
        );
        assert_eq!(p.display, Some(taffy::Display::Flex));
        assert_eq!(p.flex_direction, Some(Orientation::Horizontal));
        assert_eq!(p.justify_content, Some(taffy::AlignContent::SpaceBetween));
        assert_eq!(p.align_items, Some(taffy::AlignItems::Center));
        assert_eq!(p.flex_wrap, Some(taffy::FlexWrap::Wrap));
        assert_eq!(p.gap, Some(10.0));
    }

    #[test]
    fn background_image_single_quotes() {
        let p = parse_layout_props("background-image: url('img/btn.png')");
        assert_eq!(p.background_image, Some("img/btn.png".to_string()));
    }

    #[test]
    fn background_image_double_quotes() {
        let p = parse_layout_props("background-image: url(\"panel.png\")");
        assert_eq!(p.background_image, Some("panel.png".to_string()));
    }

    #[test]
    fn background_image_no_quotes() {
        let p = parse_layout_props("background-image: url(bg.png)");
        assert_eq!(p.background_image, Some("bg.png".to_string()));
    }

    #[test]
    fn background_shorthand_with_url() {
        let p = parse_layout_props("background: url('res/panel_bg.png')");
        assert_eq!(p.background_image, Some("res/panel_bg.png".to_string()));
    }

    #[test]
    fn background_image_with_other_props() {
        let p = parse_layout_props("width:200px;height:60px;background-image:url('btn.png')");
        assert_eq!(p.width, Some(Dimension::Px(200.0)));
        assert_eq!(p.height, Some(Dimension::Px(60.0)));
        assert_eq!(p.background_image, Some("btn.png".to_string()));
    }

    // ── Transform tests ────────────────────────────────────────────────

    #[test]
    fn transform_scale_uniform() {
        let p = parse_layout_props("transform: scale(2)");
        assert_eq!(p.scale_x, Some(2.0));
        assert_eq!(p.scale_y, Some(2.0));
    }

    #[test]
    fn transform_scale_non_uniform() {
        let p = parse_layout_props("transform: scale(1.5, 2.0)");
        assert_eq!(p.scale_x, Some(1.5));
        assert_eq!(p.scale_y, Some(2.0));
    }

    #[test]
    fn transform_scale_x_only() {
        let p = parse_layout_props("transform: scaleX(3)");
        assert_eq!(p.scale_x, Some(3.0));
        assert_eq!(p.scale_y, Some(1.0));
    }

    #[test]
    fn transform_rotate_deg() {
        let p = parse_layout_props("transform: rotate(45deg)");
        assert_eq!(p.rotation, Some(45.0));
    }

    #[test]
    fn transform_rotate_zero() {
        let p = parse_layout_props("transform: rotate(0)");
        assert_eq!(p.rotation, Some(0.0));
    }

    #[test]
    fn transform_combined_scale_rotate() {
        let p = parse_layout_props("transform: scale(1.5) rotate(15deg)");
        assert_eq!(p.scale_x, Some(1.5));
        assert_eq!(p.scale_y, Some(1.5));
        assert_eq!(p.rotation, Some(15.0));
    }

    #[test]
    fn transform_coexists_with_transform_origin() {
        let p = parse_layout_props("transform-origin: center; transform: scale(2)");
        assert_eq!(p.anchor_x, Some(0.5));
        assert_eq!(p.anchor_y, Some(0.5));
        assert_eq!(p.scale_x, Some(2.0));
        assert_eq!(p.scale_y, Some(2.0));
    }

    #[test]
    fn negative_scale() {
        let p = parse_layout_props("transform: scale(-1, 1)");
        assert_eq!(p.scale_x, Some(-1.0));
        assert_eq!(p.scale_y, Some(1.0));
    }

    // ── Opacity / Visibility / Z-index tests ───────────────────────────

    #[test]
    fn opacity_value() {
        let p = parse_layout_props("opacity: 0.5");
        assert_eq!(p.opacity, Some(0.5));
    }

    #[test]
    fn visibility_hidden() {
        let p = parse_layout_props("visibility: hidden");
        assert_eq!(p.visible, Some(false));
    }

    #[test]
    fn visibility_visible() {
        let p = parse_layout_props("visibility: visible");
        assert_eq!(p.visible, Some(true));
    }

    #[test]
    fn z_index_positive() {
        let p = parse_layout_props("z-index: 10");
        assert_eq!(p.z_order, Some(10));
    }

    #[test]
    fn z_index_negative() {
        let p = parse_layout_props("z-index: -5");
        assert_eq!(p.z_order, Some(-5));
    }

    // ── Color tests ────────────────────────────────────────────────────

    #[test]
    fn color_hex() {
        let p = parse_layout_props("color: #ff0000");
        assert_eq!(p.color, Some((255, 0, 0)));
    }

    #[test]
    fn color_named() {
        let p = parse_layout_props("color: red");
        assert_eq!(p.color, Some((255, 0, 0)));
    }

    #[test]
    fn color_rgb_function() {
        let p = parse_layout_props("color: rgb(100, 200, 50)");
        assert_eq!(p.color, Some((100, 200, 50)));
    }

    #[test]
    fn background_color_hex_with_alpha() {
        let p = parse_layout_props("background-color: #00ff0080");
        assert_eq!(p.background_color, Some((0, 255, 0, 128)));
    }

    #[test]
    fn background_color_rgb() {
        let p = parse_layout_props("background-color: rgb(100, 200, 50)");
        assert_eq!(p.background_color, Some((100, 200, 50, 255)));
    }

    #[test]
    fn background_color_rgba() {
        let p = parse_layout_props("background-color: rgba(255, 0, 0, 0.5)");
        let (r, g, b, a) = p.background_color.unwrap();
        assert_eq!((r, g, b), (255, 0, 0));
        assert!((a as i32 - 128).abs() <= 1);
    }
}
