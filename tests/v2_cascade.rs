//! M3 verification: CSS cascade pipeline.
//!
//! Covers `docs/xiangxue-css-subset.md` §1 (selectors), §13 (cascade order
//! + inheritance), and the pipeline integration with HTML parse from M2.


use xiangxue as v2;
use xiangxue::{Color, ComputedStyle, Display, Length, NodeKind, Position};

fn build(html: &str, css: &[&str]) -> v2::Document {
    let mut doc = v2::parse::html::parse(html).expect("html parses");
    v2::cascade::run(&mut doc, &[], css).expect("cascade succeeds");
    doc
}

fn build_one_rule(html: &str, css_text: &str) -> v2::Document {
    build(html, &[css_text])
}

fn computed_of<'a>(doc: &'a v2::Document, id: v2::NodeId) -> &'a ComputedStyle {
    match &doc.get(id).expect("node").kind {
        NodeKind::Element { computed, .. } => computed,
        _ => panic!("not an element"),
    }
}

fn find_by_tag(doc: &v2::Document, tag: &str) -> v2::NodeId {
    fn walk(doc: &v2::Document, id: v2::NodeId, tag: &str, out: &mut Option<v2::NodeId>) {
        if let Some(n) = doc.get(id) {
            if let NodeKind::Element { tag: t, .. } = &n.kind {
                if t == tag && out.is_none() {
                    *out = Some(id);
                }
            }
            for &c in &n.children {
                walk(doc, c, tag, out);
            }
        }
    }
    let mut out = None;
    walk(doc, doc.root, tag, &mut out);
    out.unwrap_or_else(|| panic!("no <{tag}> in document"))
}

fn find_by_id(doc: &v2::Document, id_attr: &str) -> v2::NodeId {
    fn walk(doc: &v2::Document, id: v2::NodeId, want: &str, out: &mut Option<v2::NodeId>) {
        if let Some(n) = doc.get(id) {
            if let NodeKind::Element { attrs, .. } = &n.kind {
                if attrs.get("id").map(|s| s.as_str()) == Some(want) && out.is_none() {
                    *out = Some(id);
                }
            }
            for &c in &n.children {
                walk(doc, c, want, out);
            }
        }
    }
    let mut out = None;
    walk(doc, doc.root, id_attr, &mut out);
    out.unwrap_or_else(|| panic!("no #{id_attr} in document"))
}

// ── Selector matching ──

#[test]
fn type_selector_matches() {
    let doc = build_one_rule(
        "<body><span></span><span></span></body>",
        "span { color: red }",
    );
    let span_id = find_by_tag(&doc, "span");
    let s = computed_of(&doc, span_id);
    assert_eq!(s.color, Color::Rgba(255, 0, 0, 255));
}

#[test]
fn class_selector_matches() {
    let doc = build_one_rule(
        r#"<div class="a"></div><div class="b"></div>"#,
        ".a { color: blue }",
    );
    let body = doc.get(doc.root().children[1]).unwrap();
    let div_a = doc.get(body.children[0]).unwrap();
    let div_b = doc.get(body.children[1]).unwrap();
    if let NodeKind::Element { computed, .. } = &div_a.kind {
        assert_eq!(computed.color, Color::Rgba(0, 0, 255, 255));
    }
    if let NodeKind::Element { computed, .. } = &div_b.kind {
        // .b doesn't match .a; default black.
        assert_eq!(computed.color, Color::Rgba(0, 0, 0, 255));
    }
}

#[test]
fn id_selector_matches() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { color: green }",
    );
    let id = find_by_id(&doc, "x");
    let s = computed_of(&doc, id);
    assert_eq!(s.color, Color::Rgba(0, 128, 0, 255));
}

#[test]
fn attr_exists_selector() {
    let doc = build_one_rule(
        r#"<div data-mh-widget="Button"></div><div></div>"#,
        "[data-mh-widget] { opacity: 0.5 }",
    );
    let body = doc.get(doc.root().children[1]).unwrap();
    let with_attr = doc.get(body.children[0]).unwrap();
    let without = doc.get(body.children[1]).unwrap();
    if let NodeKind::Element { computed, .. } = &with_attr.kind {
        assert!((computed.opacity - 0.5).abs() < 0.001);
    }
    if let NodeKind::Element { computed, .. } = &without.kind {
        assert!((computed.opacity - 1.0).abs() < 0.001);
    }
}

#[test]
fn attr_equals_selector() {
    let doc = build_one_rule(
        r#"<div data-x="yes"></div><div data-x="no"></div>"#,
        r#"[data-x="yes"] { opacity: 0.3 }"#,
    );
    let body = doc.get(doc.root().children[1]).unwrap();
    let yes = doc.get(body.children[0]).unwrap();
    let no = doc.get(body.children[1]).unwrap();
    if let NodeKind::Element { computed, .. } = &yes.kind {
        assert!((computed.opacity - 0.3).abs() < 0.001);
    }
    if let NodeKind::Element { computed, .. } = &no.kind {
        assert!((computed.opacity - 1.0).abs() < 0.001);
    }
}

#[test]
fn descendant_combinator() {
    let doc = build_one_rule(
        "<div><span><b></b></span></div>",
        "div b { opacity: 0.7 }",
    );
    let b = find_by_tag(&doc, "b");
    let s = computed_of(&doc, b);
    assert!((s.opacity - 0.7).abs() < 0.001);
}

#[test]
fn child_combinator_strict() {
    // div > b should NOT match if there's a span between them.
    let doc = build_one_rule(
        "<div><span><b></b></span></div>",
        "div > b { opacity: 0.7 }",
    );
    let b = find_by_tag(&doc, "b");
    let s = computed_of(&doc, b);
    // No match; opacity stays 1.0
    assert!((s.opacity - 1.0).abs() < 0.001);
}

#[test]
fn child_combinator_direct() {
    // div > b SHOULD match if b is direct child.
    let doc = build_one_rule(
        "<div><b></b></div>",
        "div > b { opacity: 0.7 }",
    );
    let b = find_by_tag(&doc, "b");
    let s = computed_of(&doc, b);
    assert!((s.opacity - 0.7).abs() < 0.001);
}

// ── Specificity / cascade order ──

#[test]
fn higher_specificity_wins() {
    let doc = build_one_rule(
        r#"<div id="x" class="c"></div>"#,
        "div { color: red } .c { color: green } #x { color: blue }",
    );
    let id = find_by_id(&doc, "x");
    let s = computed_of(&doc, id);
    // #x wins (specificity 1,0,0 > 0,1,0 > 0,0,1)
    assert_eq!(s.color, Color::Rgba(0, 0, 255, 255));
}

#[test]
fn equal_specificity_later_wins() {
    let doc = build_one_rule(
        r#"<div class="c"></div>"#,
        ".c { color: red } .c { color: green }",
    );
    let body = doc.get(doc.root().children[1]).unwrap();
    let div = doc.get(body.children[0]).unwrap();
    if let NodeKind::Element { computed, .. } = &div.kind {
        assert_eq!(computed.color, Color::Rgba(0, 128, 0, 255));
    }
}

#[test]
fn inline_style_overrides_external() {
    let doc = build_one_rule(
        r#"<div id="x" style="color: red"></div>"#,
        "#x { color: blue }",
    );
    let id = find_by_id(&doc, "x");
    let s = computed_of(&doc, id);
    assert_eq!(s.color, Color::Rgba(255, 0, 0, 255));
}

// ── Inheritance ──

#[test]
fn color_inherits_from_parent() {
    let doc = build_one_rule(
        "<div><span></span></div>",
        "div { color: red }",
    );
    let span = find_by_tag(&doc, "span");
    let s = computed_of(&doc, span);
    assert_eq!(s.color, Color::Rgba(255, 0, 0, 255));
}

#[test]
fn font_size_inherits() {
    let doc = build_one_rule(
        "<div><span></span></div>",
        "div { font-size: 24px }",
    );
    let span = find_by_tag(&doc, "span");
    let s = computed_of(&doc, span);
    assert!((s.font.size - 24.0).abs() < 0.01);
}

#[test]
fn child_overrides_inherit() {
    let doc = build_one_rule(
        "<div><span></span></div>",
        "div { color: red } span { color: blue }",
    );
    let span = find_by_tag(&doc, "span");
    let s = computed_of(&doc, span);
    assert_eq!(s.color, Color::Rgba(0, 0, 255, 255));
}

// ── Property mapping ──

#[test]
fn display_flex() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { display: flex }",
    );
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert_eq!(s.display, Display::Flex);
}

#[test]
fn position_absolute() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { position: absolute; top: 10px; left: 20px }",
    );
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert_eq!(s.position, Position::Absolute);
    assert_eq!(s.inset.top, Length::Px(10.0));
    assert_eq!(s.inset.left, Length::Px(20.0));
}

#[test]
fn width_height_px_and_percent() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { width: 200px; height: 50% }",
    );
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert_eq!(s.width, Length::Px(200.0));
    assert_eq!(s.height, Length::Percent(50.0));
}

#[test]
fn padding_shorthand_4_values() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { padding: 1px 2px 3px 4px }",
    );
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert_eq!(s.padding.top, Length::Px(1.0));
    assert_eq!(s.padding.right, Length::Px(2.0));
    assert_eq!(s.padding.bottom, Length::Px(3.0));
    assert_eq!(s.padding.left, Length::Px(4.0));
}

#[test]
fn font_shorthand_via_individuals() {
    let doc = build_one_rule(
        r#"<div id="x"></div>"#,
        "#x { font-size: 18px; font-weight: bold }",
    );
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert!((s.font.size - 18.0).abs() < 0.01);
    assert_eq!(s.font.weight, v2::FontWeight::Bold);
}

// ── Unsupported CSS surfaces as error ──

#[test]
fn media_query_returns_unsupported() {
    let mut doc = v2::parse::html::parse("<div></div>").unwrap();
    let result = v2::cascade::run(
        &mut doc,
        &[],
        &["@media (min-width: 600px) { div { color: red } }"],
    );
    assert!(matches!(
        result,
        Err(v2::LayoutError::UnsupportedCss { .. })
    ));
}

#[test]
fn hover_pseudo_returns_unsupported() {
    let mut doc = v2::parse::html::parse("<div></div>").unwrap();
    let result = v2::cascade::run(&mut doc, &[], &["div:hover { color: red }"]);
    assert!(matches!(
        result,
        Err(v2::LayoutError::UnsupportedCss { .. })
    ));
}

#[test]
fn not_selector_returns_unsupported() {
    let mut doc = v2::parse::html::parse("<div></div>").unwrap();
    let result = v2::cascade::run(&mut doc, &[], &["div:not(.foo) { color: red }"]);
    assert!(matches!(
        result,
        Err(v2::LayoutError::UnsupportedCss { .. })
    ));
}

// ── <style> tag inside HTML ──

#[test]
fn embedded_style_tag_applies() {
    let html = r#"<html><head><style>
        div { color: red }
    </style></head><body><div id="x"></div></body></html>"#;
    let doc = build(html, &[]);
    let s = computed_of(&doc, find_by_id(&doc, "x"));
    assert_eq!(s.color, Color::Rgba(255, 0, 0, 255));
}

// ── data-mh-* attrs reach attrs map without interpretation ──

#[test]
fn data_mh_attrs_in_attrs_not_interpreted() {
    let doc = build(
        r#"<div data-mh-widget="Button" data-mh-name="okBtn"></div>"#,
        &[],
    );
    let body = doc.get(doc.root().children[1]).unwrap();
    let div = doc.get(body.children[0]).unwrap();
    if let NodeKind::Element { tag, attrs, .. } = &div.kind {
        // Tag remains the HTML original — never mapped to "MhButton".
        assert_eq!(tag, "div");
        assert_eq!(attrs.get("data-mh-widget").map(|s| s.as_str()), Some("Button"));
        assert_eq!(attrs.get("data-mh-name").map(|s| s.as_str()), Some("okBtn"));
    }
}
