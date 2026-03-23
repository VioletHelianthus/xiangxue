use std::collections::HashMap;

/// Orientation for layout containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A CSS dimension value: absolute pixels or percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Px(f32),
    Percent(f32), // 0-100, CSS convention
}

/// The kind of UI widget a node represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetKind {
    Layout(Orientation),
    Button,
    Text,
    Image,
    ScrollView,
    ListView,
    TextField,
    CheckBox,
    Slider,
    ProgressBar,
    Unknown(String),
}

/// Raw CSS properties extracted from inline `style` attribute.
#[derive(Debug, Clone, Default)]
pub struct CssProperties {
    pub raw: String,
}

/// Layout properties parsed from inline CSS.
/// All coordinates are in CSS space (origin top-left).
#[derive(Debug, Clone, Default)]
pub struct LayoutProps {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub left: Option<Dimension>,
    pub top: Option<Dimension>,
    pub right: Option<Dimension>,
    pub bottom: Option<Dimension>,
    pub margin_top: Option<f32>,    // px only
    pub margin_bottom: Option<f32>,
    pub margin_left: Option<f32>,
    pub margin_right: Option<f32>,
    pub padding_top: Option<f32>,   // px only
    pub padding_bottom: Option<f32>,
    pub padding_left: Option<f32>,
    pub padding_right: Option<f32>,
    pub gap: Option<f32>,           // px only
    pub anchor_x: Option<f32>,      // 0-1, CSS space (0=left, 1=right)
    pub anchor_y: Option<f32>,      // 0-1, CSS space (0=top, 1=bottom)
    pub overflow_scroll: bool,
    pub flex_direction: Option<Orientation>,

    // Taffy layout properties (parsed via Taffy's FromStr)
    pub display: Option<taffy::Display>,
    pub position: Option<taffy::Position>,
    pub justify_content: Option<taffy::JustifyContent>,
    pub align_items: Option<taffy::AlignItems>,
    pub align_self: Option<taffy::AlignSelf>,
    pub flex_wrap: Option<taffy::FlexWrap>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Dimension>,

    // CSS Grid properties (container)
    pub grid_template_columns: Vec<taffy::GridTemplateComponent<String>>,
    pub grid_template_rows: Vec<taffy::GridTemplateComponent<String>>,
    pub grid_auto_flow: Option<taffy::GridAutoFlow>,
    pub grid_auto_rows: Vec<taffy::TrackSizingFunction>,
    pub grid_auto_columns: Vec<taffy::TrackSizingFunction>,
    // CSS Grid properties (item)
    pub grid_column_start: Option<taffy::GridPlacement<String>>,
    pub grid_column_end: Option<taffy::GridPlacement<String>>,
    pub grid_row_start: Option<taffy::GridPlacement<String>>,
    pub grid_row_end: Option<taffy::GridPlacement<String>>,
    // Gap overrides for grid
    pub column_gap: Option<f32>,
    pub row_gap: Option<f32>,

    // Visual properties parsed from CSS
    pub background_image: Option<String>, // url path from background-image: url(...)
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub rotation: Option<f32>,                     // degrees
    pub opacity: Option<f32>,                      // 0.0-1.0
    pub visible: Option<bool>,
    pub z_order: Option<i32>,
    pub color: Option<(u8, u8, u8)>,               // RGB
    pub background_color: Option<(u8, u8, u8, u8)>, // RGBA

    // Resolved layout (written by Taffy, read by backend)
    pub resolved_x: Option<f32>,
    pub resolved_y: Option<f32>,
    pub resolved_width: Option<f32>,
    pub resolved_height: Option<f32>,
}

/// A node in the UI tree.
#[derive(Debug, Clone)]
pub struct UiNode {
    pub name: String,
    pub widget: WidgetKind,
    pub children: Vec<UiNode>,
    pub attrs: HashMap<String, String>,
    pub css: CssProperties,
    pub layout: LayoutProps,
}

/// Trait for engine backends that consume the UiNode tree.
pub trait Backend {
    type Error: std::fmt::Display;
    /// Output file extension without dot (e.g., "csd", "prefab").
    fn extension(&self) -> &str;
    /// Design resolution this backend was configured with.
    fn design_size(&self) -> (f32, f32);
    /// Emit the UiNode tree as engine asset bytes.
    fn emit(&self, root: &UiNode) -> Result<Vec<u8>, Self::Error>;
}
