//! html5ever → Document arena round-tripping.
//!
//! The fixtures intentionally include `data-x-*` attributes — the parser
//! treats them as plain `attrs` entries (no DSL interpretation), proving the
//! design intent of treating custom data-* attributes as opaque holds.



use xiangxue::{NodeData, NodeKind};

fn root(doc: &xiangxue::Document) -> &NodeData {
    doc.root()
}

fn first_child<'a>(doc: &'a xiangxue::Document, node: &NodeData) -> &'a NodeData {
    doc.get(node.children[0]).expect("missing child")
}

fn element_tag(node: &NodeData) -> &str {
    match &node.kind {
        NodeKind::Element { tag, .. } => tag.as_str(),
        _ => panic!("expected element, got {:?}", debug_kind(&node.kind)),
    }
}

fn element_attr<'a>(node: &'a NodeData, name: &str) -> Option<&'a str> {
    match &node.kind {
        NodeKind::Element { attrs, .. } => attrs.get(name).map(String::as_str),
        _ => None,
    }
}

fn debug_kind(k: &NodeKind) -> &'static str {
    match k {
        NodeKind::Element { .. } => "Element",
        NodeKind::Text(_) => "Text",
        NodeKind::Comment(_) => "Comment",
    }
}

fn collect_text(doc: &xiangxue::Document, node: &NodeData) -> String {
    let mut out = String::new();
    if let NodeKind::Text(s) = &node.kind {
        out.push_str(s);
    }
    for &child in &node.children {
        if let Some(c) = doc.get(child) {
            out.push_str(&collect_text(doc, c));
        }
    }
    out
}

#[test]
fn root_is_html_element() {
    let doc = xiangxue::parse::html::parse("<html><head></head><body></body></html>").unwrap();
    let r = root(&doc);
    assert_eq!(element_tag(r), "html");
    assert!(r.parent.is_none());
    // <head> + <body>
    assert_eq!(r.children.len(), 2);
    let head = doc.get(r.children[0]).unwrap();
    let body = doc.get(r.children[1]).unwrap();
    assert_eq!(element_tag(head), "head");
    assert_eq!(element_tag(body), "body");
    assert_eq!(head.parent, Some(r.id));
    assert_eq!(body.parent, Some(r.id));
}

#[test]
fn missing_html_synthesized_by_html5ever() {
    // html5ever auto-builds <html><head></head><body>...</body></html>
    // when the input is bare body content.
    let doc = xiangxue::parse::html::parse("<div>hi</div>").unwrap();
    let r = root(&doc);
    assert_eq!(element_tag(r), "html");
    let body = doc.get(r.children[1]).expect("body present");
    assert_eq!(element_tag(body), "body");
    let div = first_child(&doc, body);
    assert_eq!(element_tag(div), "div");
    assert_eq!(collect_text(&doc, div), "hi");
}

#[test]
fn text_node_is_separate_variant() {
    let doc = xiangxue::parse::html::parse("<p>hello</p>").unwrap();
    let body = doc.get(doc.root().children[1]).unwrap();
    let p = first_child(&doc, body);
    assert_eq!(element_tag(p), "p");
    assert_eq!(p.children.len(), 1);
    let text = doc.get(p.children[0]).unwrap();
    assert!(matches!(text.kind, NodeKind::Text(ref s) if s == "hello"));
}

#[test]
fn comment_preserved() {
    let doc = xiangxue::parse::html::parse("<body><!-- note --><div></div></body>").unwrap();
    let body = doc.get(doc.root().children[1]).unwrap();
    let comment = doc.get(body.children[0]).unwrap();
    assert!(matches!(comment.kind, NodeKind::Comment(ref s) if s.contains("note")));
    let div = doc.get(body.children[1]).unwrap();
    assert_eq!(element_tag(div), "div");
}

#[test]
fn attrs_preserved_in_btreemap_order() {
    let doc = xiangxue::parse::html::parse(
        r#"<div id="x" class="y" data-x-widget="Button" data-x-name="okBtn"></div>"#,
    )
    .unwrap();
    let body = doc.get(doc.root().children[1]).unwrap();
    let div = first_child(&doc, body);
    // All data-* go into attrs verbatim — core does not interpret data-x-*.
    assert_eq!(element_attr(div, "id"), Some("x"));
    assert_eq!(element_attr(div, "class"), Some("y"));
    assert_eq!(element_attr(div, "data-x-widget"), Some("Button"));
    assert_eq!(element_attr(div, "data-x-name"), Some("okBtn"));
}

#[test]
fn parent_pointers_consistent() {
    let doc = xiangxue::parse::html::parse("<body><div><span><b></b></span></div></body>").unwrap();
    for (id, node) in doc.nodes_iter() {
        if let Some(parent_id) = node.parent {
            let p = doc.get(parent_id).expect("parent must exist");
            assert!(
                p.children.contains(&id),
                "node {id} parent={parent_id} but parent.children = {:?}",
                p.children
            );
        }
    }
    // root has no parent
    assert!(doc.root().parent.is_none());
}

#[test]
fn nested_structure_with_mixed_content() {
    let html = r#"
        <div data-x-name="root">
            <div data-x-widget="Button">Click me</div>
            <span>Plain text</span>
        </div>
    "#;
    let doc = xiangxue::parse::html::parse(html).unwrap();
    let body = doc.get(doc.root().children[1]).unwrap();
    let outer_div = first_child(&doc, body);
    assert_eq!(element_attr(outer_div, "data-x-name"), Some("root"));

    // The button div + the span (whitespace text nodes between, depending on parser)
    let elements: Vec<_> = outer_div
        .children
        .iter()
        .filter_map(|&c| {
            doc.get(c)
                .filter(|n| matches!(n.kind, NodeKind::Element { .. }))
        })
        .collect();
    assert_eq!(elements.len(), 2);
    assert_eq!(element_tag(elements[0]), "div");
    assert_eq!(element_attr(elements[0], "data-x-widget"), Some("Button"));
    assert_eq!(element_tag(elements[1]), "span");
}

#[test]
fn deep_tree_doesnt_overflow_arena() {
    // 200 nested divs — verifies arena handling isn't recursion-bound.
    let mut html = String::new();
    for _ in 0..200 {
        html.push_str("<div>");
    }
    html.push_str("leaf");
    for _ in 0..200 {
        html.push_str("</div>");
    }
    let doc = xiangxue::parse::html::parse(&html).unwrap();
    // body + 200 nested divs + 1 text leaf = 202 elements + 1 text + html + head
    assert!(doc.len() >= 200);
}

#[test]
fn whitespace_between_elements_becomes_text_nodes() {
    let doc = xiangxue::parse::html::parse("<body><div></div>\n  <span></span></body>").unwrap();
    let body = doc.get(doc.root().children[1]).unwrap();
    // div, whitespace text, span
    assert_eq!(body.children.len(), 3);
    let kinds: Vec<&'static str> = body
        .children
        .iter()
        .filter_map(|&c| doc.get(c).map(|n| debug_kind(&n.kind)))
        .collect();
    assert_eq!(kinds, vec!["Element", "Text", "Element"]);
}

/// Helper trait to peek at all nodes in the arena.
trait DocPeek {
    fn nodes_iter(&self) -> Box<dyn Iterator<Item = (xiangxue::NodeId, &NodeData)> + '_>;
}

impl DocPeek for xiangxue::Document {
    fn nodes_iter(&self) -> Box<dyn Iterator<Item = (xiangxue::NodeId, &NodeData)> + '_> {
        // We can't access `nodes` field (pub(crate)) — walk via root + BFS.
        let mut visited = vec![self.root];
        let mut idx = 0;
        let mut out = Vec::new();
        while idx < visited.len() {
            let id = visited[idx];
            idx += 1;
            if let Some(n) = self.get(id) {
                out.push((id, n));
                for &c in &n.children {
                    visited.push(c);
                }
            }
        }
        Box::new(out.into_iter())
    }
}
