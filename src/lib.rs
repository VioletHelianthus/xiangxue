pub mod cli;
pub mod display;
pub mod font;
pub mod layout;
pub mod parser;
pub mod style;
pub mod types;

pub use cli::run_cli;
pub use display::format_tree;
pub use font::FontRegistry;
pub use layout::{resolve_layout, resolve_layout_with_font};
pub use parser::parse_html;
pub use types::{Backend, CssProperties, Dimension, LayoutProps, Orientation, TextAlign, UiNode, WidgetKind};
