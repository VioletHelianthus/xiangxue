//! HTML5 parsing into a Document arena.
//!
//! Wraps html5ever's `TreeSink` to write into our arena. `ParseOpts` runs
//! with `scripting_enabled = false`, `drop_doctype = true`, and
//! `QuirksMode::NoQuirks`.
//!
//! Notes:
//! - Single-shot parse only; no incremental updates / mutator API.
//! - We accept Element / Text / Comment node kinds. ProcessingInstruction
//!   is downgraded to Comment. Template content is treated as a regular Element.
//! - The conceptual "document" node uses sentinel
//!   [`DOCUMENT_SENTINEL`](crate::document::DOCUMENT_SENTINEL); after parse
//!   the first `<html>` child becomes `Document.root`.
//! - QualNames are tracked Sink-side only — they're a parser-level concept and
//!   do not leak into the public `Document` API (which keeps tag as `String`).

use std::borrow::Cow;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{BTreeMap, HashMap};

use html5ever::ParseOpts;
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tokenizer::TokenizerOpts;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeBuilderOpts, TreeSink};
use html5ever::{Attribute, QualName};

use crate::document::{DOCUMENT_SENTINEL, Document};
use crate::error::LayoutError;
use crate::node::{NodeData, NodeId, NodeKind};

/// Parse an HTML string into a [`Document`].
pub fn parse(html: &str) -> Result<Document, LayoutError> {
    let doc = Document::new();
    let sink = Sink::new(doc);

    let document = html5ever::parse_document(sink, parse_opts())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .map_err(|e| LayoutError::HtmlParse(e.to_string()))?;

    finalize(document)
}

fn parse_opts() -> ParseOpts {
    ParseOpts {
        tokenizer: TokenizerOpts::default(),
        tree_builder: TreeBuilderOpts {
            exact_errors: false,
            scripting_enabled: false,
            iframe_srcdoc: false,
            drop_doctype: true,
            quirks_mode: QuirksMode::NoQuirks,
        },
    }
}

/// Pick the top-level `<html>` element as Document.root, or any first Element
/// if no `<html>` is present (defensive for fragment-like input).
fn finalize(mut document: Document) -> Result<Document, LayoutError> {
    let mut top_level: Vec<NodeId> = document
        .nodes
        .iter()
        .filter(|(_, n)| n.parent.is_none())
        .map(|(id, _)| id)
        .collect();
    top_level.sort_unstable();

    let html_id = top_level
        .iter()
        .copied()
        .find(|&id| matches!(
            &document.nodes[id].kind,
            NodeKind::Element { tag, .. } if tag == "html"
        ))
        .or_else(|| {
            top_level
                .iter()
                .copied()
                .find(|&id| matches!(&document.nodes[id].kind, NodeKind::Element { .. }))
        })
        .ok_or_else(|| LayoutError::HtmlParse("no root element found".into()))?;

    document.set_root(html_id);
    Ok(document)
}

/// TreeSink owning a [`Document`] under interior mutability so html5ever can
/// drive it through `&self` callbacks. `qual_names` is a parser-private map
/// from NodeId → QualName, used only for `elem_name()`. After parse the sink
/// is consumed by `finish()` and the map is dropped.
struct Sink {
    doc: RefCell<Document>,
    qual_names: RefCell<HashMap<NodeId, QualName>>,
    quirks_mode: Cell<QuirksMode>,
    errors: RefCell<Vec<Cow<'static, str>>>,
}

impl Sink {
    fn new(doc: Document) -> Self {
        Sink {
            doc: RefCell::new(doc),
            qual_names: RefCell::new(HashMap::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
            errors: RefCell::new(Vec::new()),
        }
    }

    fn doc_mut(&self) -> RefMut<'_, Document> {
        self.doc.borrow_mut()
    }

    fn doc_ref(&self) -> Ref<'_, Document> {
        self.doc.borrow()
    }
}

fn html5ever_attrs_to_btree(attrs: Vec<Attribute>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for attr in attrs {
        out.insert(attr.name.local.to_string(), attr.value.to_string());
    }
    out
}

impl TreeSink for Sink {
    type Output = Document;
    type Handle = NodeId;
    type ElemName<'a>
        = Ref<'a, QualName>
    where
        Self: 'a;

    fn finish(self) -> Self::Output {
        self.doc.into_inner()
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.errors.borrow_mut().push(msg);
    }

    fn get_document(&self) -> Self::Handle {
        DOCUMENT_SENTINEL
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        let id = *target;
        Ref::map(self.qual_names.borrow(), |m| {
            m.get(&id)
                .expect("elem_name called on a non-element handle")
        })
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let tag = name.local.to_string();
        let attrs_map = html5ever_attrs_to_btree(attrs);
        let node = NodeData::new_element(tag, attrs_map);
        let id = self.doc_mut().insert(node);
        self.qual_names.borrow_mut().insert(id, name);
        id
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        let node = NodeData::new_comment(text.to_string());
        self.doc_mut().insert(node)
    }

    fn create_pi(&self, _target: StrTendril, data: StrTendril) -> Self::Handle {
        // Downgrade processing instructions to comments — we don't model them.
        let node = NodeData::new_comment(data.to_string());
        self.doc_mut().insert(node)
    }

    fn append(&self, parent_id: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let parent = *parent_id;
        match child {
            NodeOrText::AppendNode(id) => {
                if parent == DOCUMENT_SENTINEL {
                    // Top-level node (typically <html>): no real parent. Detach
                    // from any prior parent and leave parent==None so finalize()
                    // can pick it up as a top-level node.
                    self.doc_mut().remove_node(id);
                } else {
                    self.doc_mut().append_child(parent, id);
                }
            }
            NodeOrText::AppendText(text) => {
                if parent == DOCUMENT_SENTINEL {
                    // Top-level whitespace before <html>: ignore (HTML5 allows).
                    return;
                }
                let mut doc = self.doc_mut();
                // Try to merge into the trailing Text sibling.
                let last_child = doc.get(parent).and_then(|n| n.children.last().copied());
                let merged = if let Some(last) = last_child {
                    doc.append_text_to(last, &text).is_ok()
                } else {
                    false
                };
                if !merged {
                    let node = NodeData::new_text(text.to_string());
                    let id = doc.insert(node);
                    doc.append_child(parent, id);
                }
            }
        }
    }

    fn append_before_sibling(
        &self,
        sibling_id: &Self::Handle,
        new_node: NodeOrText<Self::Handle>,
    ) {
        let sibling = *sibling_id;
        match new_node {
            NodeOrText::AppendNode(id) => {
                self.doc_mut().insert_before(sibling, id);
            }
            NodeOrText::AppendText(text) => {
                let mut doc = self.doc_mut();
                let prev = doc.previous_sibling(sibling);
                let merged = if let Some(p) = prev {
                    doc.append_text_to(p, &text).is_ok()
                } else {
                    false
                };
                if !merged {
                    let node = NodeData::new_text(text.to_string());
                    let id = doc.insert(node);
                    doc.insert_before(sibling, id);
                }
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let has_parent = self
            .doc_ref()
            .get(*element)
            .map(|n| n.parent.is_some())
            .unwrap_or(false);
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // drop_doctype=true skips this; explicit no-op for safety.
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        // Treat <template> as a regular element; its contents go inside it.
        *target
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let mut doc = self.doc_mut();
        let node = match doc.get_mut(*target) {
            Some(n) => n,
            None => return,
        };
        if let NodeKind::Element { attrs: ex, .. } = &mut node.kind {
            for attr in attrs {
                ex.entry(attr.name.local.to_string())
                    .or_insert_with(|| attr.value.to_string());
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.doc_mut().remove_node(*target);
    }

    fn reparent_children(&self, old_parent_id: &Self::Handle, new_parent_id: &Self::Handle) {
        let mut doc = self.doc_mut();
        let children: Vec<NodeId> = doc
            .get(*old_parent_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            doc.append_child(*new_parent_id, child);
        }
    }
}
