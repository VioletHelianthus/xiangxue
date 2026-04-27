use std::cell::Cell;

use slab::Slab;

use crate::box_model::Size;
use crate::font::FontProvider;
use crate::node::{NodeData, NodeId};

/// Sentinel NodeId returned by HTML parser sinks for the "document" handle —
/// representing the conceptual Document node above the `<html>` element. We do
/// not store it in the arena; instead the parser tracks top-level children
/// against this sentinel and the first `<html>` element among them becomes
/// `Document.root`.
pub(crate) const DOCUMENT_SENTINEL: NodeId = usize::MAX;

/// HTML+CSS arena. Uses `Box<Slab<...>>` so node addresses stay stable
/// across inserts.
pub struct Document {
    /// The root element NodeId. Valid only when `nodes` is non-empty.
    pub root: NodeId,
    pub(crate) nodes: Box<Slab<NodeData>>,
    /// Transient pointer to the active FontProvider during a layout solve().
    /// SAFETY: set only inside `layout::solve()` for the duration of the
    /// Taffy compute pass, then reset to `None`. Layout-time leaf measurement
    /// dereferences via [`with_font_provider`].
    pub(crate) fonts: Cell<Option<*const dyn FontProvider>>,
}

impl Document {
    /// Build a truly empty Document. The parser fills it and calls
    /// [`set_root`] afterwards.
    pub(crate) fn new() -> Self {
        Document {
            root: 0,
            nodes: Box::new(Slab::new()),
            fonts: Cell::new(None),
        }
    }

    /// Borrow the active FontProvider during layout. Returns `None` outside
    /// the `layout::solve()` scope, where the caller installs the pointer
    /// via `doc.fonts.set(...)` and clears it on exit.
    pub(crate) fn font_provider(&self) -> Option<&dyn FontProvider> {
        // SAFETY: pointer is installed by `layout::solve()` for the duration
        // of the Taffy compute pass; the FontProvider outlives that pass.
        self.fonts.get().map(|ptr| unsafe { &*ptr })
    }

    pub fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(id)
    }

    pub fn root(&self) -> &NodeData {
        &self.nodes[self.root]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn set_root(&mut self, id: NodeId) {
        self.root = id;
    }

    pub(crate) fn insert(&mut self, mut node: NodeData) -> NodeId {
        // Reserve the slot first so we can patch the id.
        let id = self.nodes.vacant_key();
        node.id = id;
        let inserted = self.nodes.insert(node);
        debug_assert_eq!(inserted, id);
        id
    }

    pub(crate) fn append_child(&mut self, parent: NodeId, child: NodeId) {
        // Detach from previous parent if any.
        if let Some(prev_parent) = self.nodes.get(child).and_then(|n| n.parent) {
            if let Some(p) = self.nodes.get_mut(prev_parent) {
                p.children.retain(|&c| c != child);
            }
        }
        if let Some(c) = self.nodes.get_mut(child) {
            c.parent = Some(parent);
        }
        if let Some(p) = self.nodes.get_mut(parent) {
            p.children.push(child);
        }
    }

    pub(crate) fn insert_before(&mut self, sibling: NodeId, new_node: NodeId) {
        // Detach new_node from its current parent if any.
        if let Some(prev_parent) = self.nodes.get(new_node).and_then(|n| n.parent) {
            if let Some(p) = self.nodes.get_mut(prev_parent) {
                p.children.retain(|&c| c != new_node);
            }
        }
        let parent = match self.nodes.get(sibling).and_then(|n| n.parent) {
            Some(p) => p,
            None => return,
        };
        if let Some(n) = self.nodes.get_mut(new_node) {
            n.parent = Some(parent);
        }
        if let Some(p) = self.nodes.get_mut(parent) {
            let pos = p
                .children
                .iter()
                .position(|&c| c == sibling)
                .unwrap_or(p.children.len());
            p.children.insert(pos, new_node);
        }
    }

    pub(crate) fn remove_node(&mut self, id: NodeId) {
        let parent_opt = self.nodes.get(id).and_then(|n| n.parent);
        if let Some(parent) = parent_opt {
            if let Some(p) = self.nodes.get_mut(parent) {
                p.children.retain(|&c| c != id);
            }
        }
        if let Some(n) = self.nodes.get_mut(id) {
            n.parent = None;
        }
    }

    /// Returns the previous sibling of `id`, if any.
    pub(crate) fn previous_sibling(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.nodes.get(id)?.parent?;
        let p = self.nodes.get(parent)?;
        let pos = p.children.iter().position(|&c| c == id)?;
        if pos == 0 {
            None
        } else {
            Some(p.children[pos - 1])
        }
    }

    /// Append `text` to the trailing characters of a Text node. Returns Err if
    /// the node isn't a Text node.
    pub(crate) fn append_text_to(&mut self, id: NodeId, text: &str) -> Result<(), ()> {
        match self.nodes.get_mut(id).map(|n| &mut n.kind) {
            Some(crate::node::NodeKind::Text(s)) => {
                s.push_str(text);
                Ok(())
            }
            _ => Err(()),
        }
    }
}

pub struct LayoutTree {
    pub document: Document,
    pub viewport: Size,
}
