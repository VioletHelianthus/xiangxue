//! Layout pass: bridge our [`Document`] to Taffy.
//!
//! Phase 3 (`flush_styles`): walks every Element and converts its
//! [`ComputedStyle`] into a `taffy::Style` cached on the node.
//!
//! Phase 4 (`solve`): runs `taffy::compute_root_layout` then
//! `taffy::round_layout`, capturing both the unrounded (subpixel) and final
//! (pixel-aligned) box rectangles into NodeData. This split mirrors Blitz's
//! `unrounded_layout` / `final_layout` (blitz-extraction §2 — important for
//! game UI to avoid 1-pixel error accumulation across nested containers).
//!
//! Document directly implements `taffy::TraversePartialTree` /
//! `LayoutPartialTree` / `RoundTree` and friends. We do **not** maintain a
//! second `taffy::TaffyTree` — our arena IS the Taffy tree
//! (blitz-extraction §4, the key pattern that eliminates v1 layout.rs's
//! complexity).

mod style_to_taffy;
mod text_measure;

use std::slice;

use taffy::{
    self, AvailableSpace, CacheTree, Layout, LayoutBlockContainer, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId as TafNodeId,
    RoundTree, Size, Style, TraversePartialTree, TraverseTree, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_leaf_layout,
    compute_root_layout, round_layout,
};

use crate::box_model::{BoxModel, Sides};
use crate::document::Document;
use crate::error::LayoutError;
use crate::font::FontProvider;
use crate::node::{NodeId, NodeKind};
use crate::pipeline::LayoutOptions;
use crate::style::Display;

/// Convert each Element's [`ComputedStyle`] to a `taffy::Style` cached on the
/// node. Comment and whitespace-only Text nodes are forced to
/// `display: none` so Taffy doesn't treat them as zero-size flex/grid items
/// (which would consume gaps and align-items rules between real children).
pub fn flush_styles(doc: &mut Document) -> Result<(), LayoutError> {
    let ids: Vec<NodeId> = (0..doc.nodes.capacity()).collect();
    for id in ids {
        let style: Option<Style> = match doc.get(id).map(|n| &n.kind) {
            Some(NodeKind::Element { computed, .. }) => Some(style_to_taffy::convert(computed)),
            Some(NodeKind::Text(s)) => {
                if s.chars().all(char::is_whitespace) {
                    let mut hidden = Style::DEFAULT;
                    hidden.display = taffy::Display::None;
                    Some(hidden)
                } else {
                    None // Real text: keep default style for M5 measurement.
                }
            }
            Some(NodeKind::Comment(_)) => {
                let mut hidden = Style::DEFAULT;
                hidden.display = taffy::Display::None;
                Some(hidden)
            }
            None => None,
        };
        if let (Some(s), Some(node)) = (style, doc.get_mut(id)) {
            node.taffy_style = s;
        }
    }
    Ok(())
}

/// Solve layout. Calls Taffy to compute box positions, populating
/// `box_subpixel` (after compute_root_layout) and `box_pixel`
/// (after round_layout) on each Element node.
///
/// Installs the FontProvider on the Document for the duration of the Taffy
/// pass so leaf Text nodes can be measured (see `text_measure`).
pub fn solve(
    doc: &mut Document,
    opts: &LayoutOptions,
    fonts: &dyn FontProvider,
) -> Result<(), LayoutError> {
    let root = doc.root;
    let root_size = opts.viewport;

    let available = Size {
        width: AvailableSpace::Definite(root_size.width),
        height: AvailableSpace::Definite(root_size.height),
    };

    // Install the FontProvider via Cell. The Cell stores
    // `*const (dyn FontProvider + 'static)` (dyn defaults to `+ 'static`),
    // but our caller's reference is shorter-lived. We transmute to lengthen
    // the lifetime — safe because we clear the Cell before this function
    // returns, before the caller's borrow could become invalid.
    let fonts_ptr: *const (dyn FontProvider + 'static) =
        unsafe { core::mem::transmute(fonts as *const dyn FontProvider) };
    doc.fonts.set(Some(fonts_ptr));

    compute_root_layout(doc, taffy_id(root), available);
    round_layout(doc, taffy_id(root));

    doc.fonts.set(None);
    Ok(())
}

#[inline]
fn taffy_id(id: NodeId) -> TafNodeId {
    TafNodeId::from(id as u64)
}

#[inline]
fn from_taffy_id(id: TafNodeId) -> NodeId {
    Into::<u64>::into(id) as NodeId
}

fn taffy_layout_to_box(layout: &Layout) -> BoxModel {
    BoxModel {
        x: layout.location.x,
        y: layout.location.y,
        width: layout.size.width,
        height: layout.size.height,
        padding: Sides {
            top: layout.padding.top,
            right: layout.padding.right,
            bottom: layout.padding.bottom,
            left: layout.padding.left,
        },
        border: Sides {
            top: layout.border.top,
            right: layout.border.right,
            bottom: layout.border.bottom,
            left: layout.border.left,
        },
        margin: Sides::default(),
        scroll: Some(crate::box_model::Size::new(
            layout.content_size.width,
            layout.content_size.height,
        )),
    }
}

// ─── Taffy traits ───────────────────────────────────────────────────────────

/// Iterator yielding `taffy::NodeId` values from our `Vec<NodeId>` children.
pub struct ChildIter<'a> {
    inner: slice::Iter<'a, NodeId>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = TafNodeId;
    #[inline]
    fn next(&mut self) -> Option<TafNodeId> {
        self.inner.next().copied().map(taffy_id)
    }
}

impl TraversePartialTree for Document {
    type ChildIter<'a> = ChildIter<'a>;

    fn child_ids(&self, parent_node_id: TafNodeId) -> Self::ChildIter<'_> {
        let id = from_taffy_id(parent_node_id);
        match self.get(id) {
            Some(node) => ChildIter {
                inner: node.children.iter(),
            },
            None => ChildIter {
                inner: [].iter(),
            },
        }
    }

    fn child_count(&self, parent_node_id: TafNodeId) -> usize {
        self.get(from_taffy_id(parent_node_id))
            .map(|n| n.children.len())
            .unwrap_or(0)
    }

    fn get_child_id(&self, parent_node_id: TafNodeId, child_index: usize) -> TafNodeId {
        let id = from_taffy_id(parent_node_id);
        let child = self.get(id).expect("parent exists").children[child_index];
        taffy_id(child)
    }
}

impl TraverseTree for Document {}

impl LayoutPartialTree for Document {
    type CoreContainerStyle<'a> = &'a Style;
    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: TafNodeId) -> &Style {
        &self.get(from_taffy_id(node_id)).expect("node").taffy_style
    }

    fn set_unrounded_layout(&mut self, node_id: TafNodeId, layout: &Layout) {
        let id = from_taffy_id(node_id);
        if let Some(node) = self.get_mut(id) {
            node.box_subpixel = taffy_layout_to_box(layout);
        }
    }

    fn compute_child_layout(&mut self, node_id: TafNodeId, inputs: LayoutInput) -> LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            tree.compute_child_layout_internal(node_id, inputs)
        })
    }
}

impl Document {
    fn compute_child_layout_internal(
        &mut self,
        node_id: TafNodeId,
        inputs: LayoutInput,
    ) -> LayoutOutput {
        let id = from_taffy_id(node_id);

        // Honour `display: none` regardless of node kind. flush_styles sets
        // this on Comments and whitespace-only Text; cascade may set it on
        // Elements explicitly.
        if self
            .get(id)
            .map(|n| n.taffy_style.display == taffy::Display::None)
            .unwrap_or(false)
        {
            return LayoutOutput::HIDDEN;
        }

        let kind_summary = match self.get(id).map(|n| &n.kind) {
            Some(NodeKind::Element { computed, .. }) => KindSummary::Element(computed.display),
            Some(NodeKind::Text(_)) => KindSummary::Text,
            Some(NodeKind::Comment(_)) | None => KindSummary::Other,
        };
        match kind_summary {
            KindSummary::Element(Display::None) => LayoutOutput::HIDDEN,
            KindSummary::Element(Display::Flex) => compute_flexbox_layout(self, node_id, inputs),
            KindSummary::Element(Display::Grid) => compute_grid_layout(self, node_id, inputs),
            KindSummary::Element(Display::Block) => {
                if self
                    .get(id)
                    .map(|n| n.children.is_empty())
                    .unwrap_or(true)
                {
                    // Empty Element leaf: respect width/height from style.
                    let style = self.get(id).map(|n| n.taffy_style.clone()).unwrap_or_default();
                    compute_leaf_layout(inputs, &style, |_, _| 0.0, |_, _| Size::ZERO)
                } else {
                    compute_block_layout(self, node_id, inputs, None)
                }
            }
            KindSummary::Text => text_measure::compute_text_leaf(self, id, inputs),
            KindSummary::Other => LayoutOutput::HIDDEN,
        }
    }
}

enum KindSummary {
    Element(Display),
    Text,
    Other,
}

impl CacheTree for Document {
    fn cache_get(&self, node_id: TafNodeId, inputs: &LayoutInput) -> Option<LayoutOutput> {
        let id = from_taffy_id(node_id);
        self.get(id).and_then(|n| n.taffy_cache.get(inputs))
    }

    fn cache_store(
        &mut self,
        node_id: TafNodeId,
        inputs: &LayoutInput,
        layout_output: LayoutOutput,
    ) {
        let id = from_taffy_id(node_id);
        if let Some(n) = self.get_mut(id) {
            n.taffy_cache.store(inputs, layout_output);
        }
    }

    fn cache_clear(&mut self, node_id: TafNodeId) {
        let id = from_taffy_id(node_id);
        if let Some(n) = self.get_mut(id) {
            n.taffy_cache.clear();
        }
    }
}

impl LayoutBlockContainer for Document {
    type BlockContainerStyle<'a> = &'a Style;
    type BlockItemStyle<'a> = &'a Style;

    fn get_block_container_style(&self, node_id: TafNodeId) -> &Style {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: TafNodeId) -> &Style {
        self.get_core_container_style(child_node_id)
    }
}

impl LayoutFlexboxContainer for Document {
    type FlexboxContainerStyle<'a> = &'a Style;
    type FlexboxItemStyle<'a> = &'a Style;

    fn get_flexbox_container_style(&self, node_id: TafNodeId) -> &Style {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: TafNodeId) -> &Style {
        self.get_core_container_style(child_node_id)
    }
}

impl LayoutGridContainer for Document {
    type GridContainerStyle<'a> = &'a Style;
    type GridItemStyle<'a> = &'a Style;

    fn get_grid_container_style(&self, node_id: TafNodeId) -> &Style {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: TafNodeId) -> &Style {
        self.get_core_container_style(child_node_id)
    }
}

impl RoundTree for Document {
    fn get_unrounded_layout(&self, node_id: TafNodeId) -> Layout {
        let id = from_taffy_id(node_id);
        match self.get(id) {
            Some(node) => Layout {
                order: 0,
                location: taffy::Point {
                    x: node.box_subpixel.x,
                    y: node.box_subpixel.y,
                },
                size: Size {
                    width: node.box_subpixel.width,
                    height: node.box_subpixel.height,
                },
                content_size: node
                    .box_subpixel
                    .scroll
                    .map(|s| Size { width: s.width, height: s.height })
                    .unwrap_or(Size { width: 0.0, height: 0.0 }),
                scrollbar_size: Size::ZERO,
                padding: taffy::Rect {
                    top: node.box_subpixel.padding.top,
                    right: node.box_subpixel.padding.right,
                    bottom: node.box_subpixel.padding.bottom,
                    left: node.box_subpixel.padding.left,
                },
                border: taffy::Rect {
                    top: node.box_subpixel.border.top,
                    right: node.box_subpixel.border.right,
                    bottom: node.box_subpixel.border.bottom,
                    left: node.box_subpixel.border.left,
                },
                margin: taffy::Rect::default(),
            },
            None => Layout::with_order(0),
        }
    }

    fn set_final_layout(&mut self, node_id: TafNodeId, layout: &Layout) {
        let id = from_taffy_id(node_id);
        if let Some(node) = self.get_mut(id) {
            node.box_pixel = taffy_layout_to_box(layout);
        }
    }
}
