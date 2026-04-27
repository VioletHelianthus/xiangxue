use std::collections::BTreeMap;

use crate::box_model::BoxModel;
use crate::style::ComputedStyle;

pub type NodeId = usize;

/// HTML5 node kinds preserved by the parser (CSS subset §2 & redesign §5.3).
/// `Element` carries `tag` (always the HTML original tag, never widget-mapped),
/// `attrs` (all `data-*` and others raw — core does not interpret), and
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
/// `box_subpixel` and `box_pixel` mirror Blitz's `unrounded_layout` /
/// `final_layout` split (blitz-extraction §2): subpixel-precise vs
/// pixel-aligned. Game UI rounds to integer pixels but subpixel is kept
/// to avoid 1px error accumulation across nested containers.
pub struct NodeData {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub box_subpixel: BoxModel,
    pub box_pixel: BoxModel,
    pub taffy_cache: taffy::Cache,
    pub taffy_style: taffy::Style,
    // Reserved for ⏸ pseudo-element support. Do not implement in v2 first
    // release; presence of these fields prevents NodeData layout churn when
    // ::before / ::after is later promoted to ✅ (CSS subset §1).
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
