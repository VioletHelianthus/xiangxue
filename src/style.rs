use crate::box_model::Sides;
use crate::font::{FontStyle, FontWeight};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Block,
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Rgba(u8, u8, u8, u8),
    /// `currentColor` keyword; resolved during cascade by inheriting `color`.
    CurrentColor,
}

impl Default for Color {
    fn default() -> Self {
        Color::Rgba(0, 0, 0, 255)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Px(f32),
    Percent(f32),
    Auto,
}

impl Default for Length {
    fn default() -> Self {
        Length::Auto
    }
}

impl Length {
    pub fn px(v: f32) -> Self {
        Length::Px(v)
    }
    pub fn percent(v: f32) -> Self {
        Length::Percent(v)
    }
}

#[derive(Debug, Clone)]
pub struct Font {
    pub family: String,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub line_height: f32,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            family: "sans-serif".into(),
            size: 16.0,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            line_height: 19.2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Background {
    pub color: Option<Color>,
    /// Raw `url(...)` string from `background-image`. xiangxue does not load images.
    pub image_url: Option<String>,
    pub repeat: Option<String>,
    pub position: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyleKind {
    #[default]
    None,
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Default)]
pub struct BorderStyle {
    pub width: f32,
    pub kind: BorderStyleKind,
    pub color: Option<Color>,
    /// Px radius; zero means square.
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformOp {
    Translate { x: Length, y: Length },
    Rotate(f32),
    Scale(f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

pub type AlignSelf = AlignItems;
pub type AlignContent = JustifyContent;

#[derive(Debug, Clone)]
pub struct FlexProps {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub align_content: Option<AlignContent>,
    pub grow: f32,
    pub shrink: f32,
    pub basis: Length,
}

impl Default for FlexProps {
    fn default() -> Self {
        FlexProps {
            direction: FlexDirection::Row,
            wrap: FlexWrap::NoWrap,
            justify_content: None,
            align_items: None,
            align_self: None,
            align_content: None,
            grow: 0.0,
            shrink: 1.0,
            basis: Length::Auto,
        }
    }
}

/// A single grid track size. Mirrors CSS `<track-breadth>` plus the auto/
/// content keywords. `Px`/`Percent` carry their value verbatim; `Fr` is
/// flex-factor; `Auto`/`MinContent`/`MaxContent` are intrinsic keywords.
#[derive(Debug, Clone, PartialEq)]
pub enum GridTrackSize {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
}

impl Default for GridTrackSize {
    fn default() -> Self {
        GridTrackSize::Auto
    }
}

/// A grid track entry. A bare track size like `100px` is `min == max`; the
/// `minmax(a, b)` function distinguishes the two.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridTrack {
    pub min: GridTrackSize,
    pub max: GridTrackSize,
}

impl GridTrack {
    pub fn from_breadth(breadth: GridTrackSize) -> Self {
        Self { min: breadth.clone(), max: breadth }
    }
}

/// One slot in `grid-template-columns` / `-rows`: either a single track or a
/// `repeat(count, ...)` group.
#[derive(Debug, Clone, PartialEq)]
pub enum GridTemplateComponent {
    Single(GridTrack),
    Repeat { count: GridRepeatCount, tracks: Vec<GridTrack> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridRepeatCount {
    Count(u16),
    AutoFill,
    AutoFit,
}

/// Named-area grid. `columns * row_count == areas.len()`. `None` slots
/// represent the `.` token (unnamed cell).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridTemplateAreas {
    pub columns: u32,
    pub areas: Vec<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

/// Grid placement endpoint (`grid-row-start`, `grid-column-end`, etc.).
/// `Named` is for named-area references; `Span` is `span N`.
#[derive(Debug, Clone, PartialEq)]
pub enum GridLine {
    Auto,
    Index(i16),
    Named(String),
    Span(u16),
}

impl Default for GridLine {
    fn default() -> Self {
        GridLine::Auto
    }
}

/// Grid container properties (CSS subset §3.6).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridProps {
    pub template_columns: Vec<GridTemplateComponent>,
    pub template_rows: Vec<GridTemplateComponent>,
    pub template_areas: Option<GridTemplateAreas>,
    pub auto_columns: Vec<GridTrack>,
    pub auto_rows: Vec<GridTrack>,
    pub auto_flow: GridAutoFlow,
}

#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    // Display & flow
    pub display: Display,
    pub position: Position,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Typography
    pub font: Font,
    pub color: Color,
    pub text_align: TextAlign,

    // Visual
    pub background: Background,
    pub border: BorderStyle,
    pub opacity: f32,
    pub visibility: Visibility,
    pub z_index: Option<i32>,

    // Transforms
    pub transforms: Vec<TransformOp>,

    // Box dimensions (CSS subset §3.1)
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    pub padding: Sides<Length>,
    pub margin: Sides<Length>,
    pub inset: Sides<Length>,

    // Gap (used for flex/grid containers; CSS subset §5/§6)
    pub gap_x: Length,
    pub gap_y: Length,

    // Layout-mode properties (CSS subset §5/§6)
    pub flex: Option<FlexProps>,
    pub grid: Option<GridProps>,

    // Grid item placement (per-element, not container; CSS subset §3.6)
    pub grid_column: (GridLine, GridLine),
    pub grid_row: (GridLine, GridLine),
}

impl ComputedStyle {
    /// Initial-value ComputedStyle per CSS spec. Critically:
    /// - margin / padding default to 0 (not auto)
    /// - width / height / min-* default to auto
    /// - max-* default to auto (= unconstrained)
    /// - inset (top/right/bottom/left) defaults to auto
    /// - opacity defaults to 1.0
    pub fn initial() -> Self {
        let zero_sides = Sides {
            top: Length::Px(0.0),
            right: Length::Px(0.0),
            bottom: Length::Px(0.0),
            left: Length::Px(0.0),
        };
        Self {
            margin: zero_sides,
            padding: zero_sides,
            opacity: 1.0,
            gap_x: Length::Px(0.0),
            gap_y: Length::Px(0.0),
            ..Default::default()
        }
    }
}
