use std::collections::BTreeMap;

use crate::box_model::BoxModel;
use crate::style::ComputedStyle;

pub type NodeId = usize;

/// HTML5 node kinds preserved by the parser. `Element` carries `tag`
/// (always the HTML original tag), `attrs` (all `data-*` and others raw —
/// the engine does not interpret application-defined attributes), and
/// `computed` filled by cascade.
#[derive(Debug, Clone)]
pub enum NodeKind {
    Element {
        tag: String,
        attrs: BTreeMap<String, String>,
        computed: ComputedStyle,
    },
    Text(String),
    /// Preserved so backends can faithfully roundtrip the source DOM.
    /// Skipped by layout.
    Comment(String),
}

impl NodeKind {
    pub fn is_element(&self) -> bool {
        matches!(self, NodeKind::Element { .. })
    }
    pub fn is_text(&self) -> bool {
        matches!(self, NodeKind::Text(_))
    }
    pub fn is_comment(&self) -> bool {
        matches!(self, NodeKind::Comment(_))
    }
}

/// A node in the Document arena.
///
/// `box_subpixel` and `box_pixel` form a subpixel-precise / pixel-aligned
/// split. Pixel-aligned coordinates are convenient for renderers, while
/// the subpixel form is kept so 1px rounding errors do not accumulate
/// across nested containers.
pub struct NodeData {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub box_subpixel: BoxModel,
    pub box_pixel: BoxModel,
    pub taffy_cache: taffy::Cache,
    pub taffy_style: taffy::Style,
    // Reserved for future pseudo-element support (`::before` / `::after`).
    // Slot kept here so adding them later does not perturb NodeData layout.
    // pub before: Option<NodeId>,
    // pub after: Option<NodeId>,
}

impl NodeData {
    /// Build a NodeData with id=0; `Document::insert` patches the id field
    /// to the actual slab key on insertion.
    pub(crate) fn new_element(tag: String, attrs: BTreeMap<String, String>) -> Self {
        NodeData {
            id: 0,
            parent: None,
            children: Vec::new(),
            kind: NodeKind::Element {
                tag,
                attrs,
                computed: ComputedStyle::initial(),
            },
            box_subpixel: BoxModel::default(),
            box_pixel: BoxModel::default(),
            taffy_cache: taffy::Cache::new(),
            taffy_style: taffy::Style::default(),
        }
    }

    pub(crate) fn new_text(content: String) -> Self {
        NodeData {
            id: 0,
            parent: None,
            children: Vec::new(),
            kind: NodeKind::Text(content),
            box_subpixel: BoxModel::default(),
            box_pixel: BoxModel::default(),
            taffy_cache: taffy::Cache::new(),
            taffy_style: taffy::Style::default(),
        }
    }

    pub(crate) fn new_comment(content: String) -> Self {
        NodeData {
            id: 0,
            parent: None,
            children: Vec::new(),
            kind: NodeKind::Comment(content),
            box_subpixel: BoxModel::default(),
            box_pixel: BoxModel::default(),
            taffy_cache: taffy::Cache::new(),
            taffy_style: taffy::Style::default(),
        }
    }
}
