//! xiangxue: pure HTML+CSS layout engine.
//!
//! Inputs: HTML + CSS strings, viewport, optional `FontProvider`.
//! Output: a fully laid-out `Document` tree where every element has its
//! computed style and resolved box geometry, ready for a downstream
//! consumer to walk and emit whatever target format it cares about.

pub mod box_model;
pub mod document;
pub mod error;
pub mod font;
pub mod node;
pub mod pipeline;
pub mod style;

pub mod cascade;
pub mod layout;
pub mod parse;

pub use box_model::{BoxModel, Sides, Size};

/// Re-export of the underlying taffy crate so downstream backends can
/// speak taffy types without taking a separate dependency.
pub use ::taffy;
pub use document::{Document, LayoutTree};
pub use error::{LayoutError, SourceSpan};
pub use font::{FontProvider, FontQuery, FontStyle, FontWeight, NoOpFontProvider, TextMetrics};
pub use node::{NodeData, NodeId, NodeKind};
pub use pipeline::{Engine, LayoutOptions, StylesheetId, layout};
pub use style::{
    AlignContent, AlignItems, AlignSelf, Background, BorderStyle, BorderStyleKind, Color,
    ComputedStyle, Display, FlexDirection, FlexProps, FlexWrap, Font, GridAutoFlow, GridLine,
    GridProps, GridRepeatCount, GridTemplateAreas, GridTemplateComponent, GridTrack,
    GridTrackSize, JustifyContent, Length, Overflow, Position, TextAlign, TransformOp, Visibility,
};
