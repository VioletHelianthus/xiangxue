//! M5 verification: FontProvider trait + text node measurement.
//!
//! Uses a deterministic mock FontProvider that returns dimensions purely as a
//! function of input, so layout assertions are stable.


use std::sync::Mutex;

use xiangxue as v2;
use xiangxue::{
    self, FontProvider, FontQuery, FontStyle, FontWeight, NodeKind, TextMetrics,
};

/// Deterministic mock: width = chars * size * 0.6, height = size * 1.2.
/// Records every measure call so tests can assert what got measured.
struct MockFontProvider {
    calls: Mutex<Vec<MockCall>>,
}

#[derive(Debug, Clone)]
struct MockCall {
    text: String,
    family: String,
    size: f32,
    weight: FontWeight,
    style: FontStyle,
}

impl MockFontProvider {
    fn new() -> Self {
        MockFontProvider {
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<MockCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl FontProvider for MockFontProvider {
    fn measure(&self, text: &str, query: &FontQuery<'_>) -> TextMetrics {
        self.calls.lock().unwrap().push(MockCall {
            text: text.to_string(),
            family: query.family.to_string(),
            size: query.size,
            weight: query.weight,
            style: query.style,
        });
        let width = text.chars().count() as f32 * query.size * 0.6;
        let height = query.size * 1.2;
        TextMetrics {
            width,
            height,
            ascent: query.size,
            descent: query.size * 0.2,
        }
    }

    fn has_face(&self, _family: &str, _weight: FontWeight, _style: FontStyle) -> bool {
        true
    }
}

fn run(html: &str, css: &[&str], fp: &dyn FontProvider) -> v2::Document {
    let mut doc = v2::parse::html::parse(html).unwrap();
    v2::cascade::run(&mut doc, &[], css).unwrap();
    v2::layout::flush_styles(&mut doc).unwrap();
    let opts = v2::LayoutOptions {
        viewport: v2::Size::new(800.0, 600.0),
        ..Default::default()
    };
    v2::layout::solve(&mut doc, &opts, fp).unwrap();
    doc
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
    out.unwrap_or_else(|| panic!("no #{id_attr}"))
}

fn close(actual: f32, expected: f32, tol: f32) -> bool {
    (actual - expected).abs() <= tol
}

// ── FontProvider is consulted ──

#[test]
fn font_provider_called_for_text() {
    let fp = MockFontProvider::new();
    run(
        r#"<div id="x" style="font-size: 16px"><span>hello</span></div>"#,
        &[],
        &fp,
    );
    let calls = fp.calls();
    assert!(
        calls.iter().any(|c| c.text == "hello"),
        "FontProvider was not called for 'hello'; saw {:?}",
        calls
    );
}

#[test]
fn font_query_carries_parent_size_and_family() {
    let fp = MockFontProvider::new();
    run(
        r#"<div style="font-size: 24px; font-family: 'Helvetica'"><span>hi</span></div>"#,
        &[],
        &fp,
    );
    let call = fp.calls().into_iter().find(|c| c.text == "hi").expect("hi measured");
    assert_eq!(call.family, "Helvetica");
    assert!(close(call.size, 24.0, 0.01));
}

#[test]
fn font_weight_and_style_inherited() {
    let fp = MockFontProvider::new();
    run(
        r#"<div style="font-weight: bold; font-style: italic"><span>x</span></div>"#,
        &[],
        &fp,
    );
    let call = fp.calls().into_iter().find(|c| c.text == "x").expect("x measured");
    assert_eq!(call.weight, FontWeight::Bold);
    assert_eq!(call.style, FontStyle::Italic);
}

// ── Text size enters layout ──

#[test]
fn text_node_box_matches_measure_result() {
    // Wrap in absolute-positioned div so it doesn't get stretched by the
    // ancestor block chain (body / html). Inside the absolute box use flex
    // so the text leaf reports its intrinsic measure as the layout size.
    let fp = MockFontProvider::new();
    let doc = run(
        r#"<div id="wrap" style="position: absolute; display: flex; font-size: 20px"><span id="s">abcde</span></div>"#,
        &[],
        &fp,
    );
    // width = 5 * 20 * 0.6 = 60, height = 20 * 1.2 = 24
    let span = find_by_id(&doc, "s");
    let bx = &doc.get(span).unwrap().box_pixel;
    assert!(close(bx.width, 60.0, 1.0), "span width = {}", bx.width);
    assert!(close(bx.height, 24.0, 1.0), "span height = {}", bx.height);
}

#[test]
fn parent_size_grows_to_fit_text() {
    let fp = MockFontProvider::new();
    let doc = run(
        r#"<div style="position: absolute; display: flex; font-size: 16px"><span id="s">hello</span></div>"#,
        &[],
        &fp,
    );
    // 5 chars * 16 * 0.6 = 48
    let span = find_by_id(&doc, "s");
    let bx = &doc.get(span).unwrap().box_pixel;
    assert!(close(bx.width, 48.0, 1.0), "span width = {}", bx.width);
}

// ── No-op provider ──

#[test]
fn noop_font_provider_used_by_one_shot_layout() {
    // The xiangxue::layout(...) one-shot uses NoOpFontProvider internally
    // (text.chars * size * 0.6, size * 1.2 — same shape as our mock).
    let opts = v2::LayoutOptions {
        viewport: v2::Size::new(400.0, 300.0),
        ..Default::default()
    };
    let tree = v2::layout(
        r#"<div style="position: absolute; display: flex; font-size: 20px"><span id="s">xy</span></div>"#,
        &[],
        &opts,
    )
    .unwrap();
    let span = find_by_id(&tree.document, "s");
    let bx = &tree.document.get(span).unwrap().box_pixel;
    // 2 chars * 20 * 0.6 = 24
    assert!(close(bx.width, 24.0, 1.0), "span width = {}", bx.width);
}

// ── Whitespace nodes still skipped ──

#[test]
fn whitespace_text_not_measured() {
    let fp = MockFontProvider::new();
    run(
        r#"<div style="display: flex">
             <span>real</span>
             <span>text</span>
           </div>"#,
        &[],
        &fp,
    );
    let calls = fp.calls();
    // Only "real" and "text" should be measured; whitespace between them
    // (which would be \n followed by spaces) must not appear.
    assert!(calls.iter().any(|c| c.text == "real"));
    assert!(calls.iter().any(|c| c.text == "text"));
    for call in &calls {
        assert!(
            !call.text.chars().all(char::is_whitespace),
            "FontProvider was called with whitespace-only text: {:?}",
            call.text
        );
    }
}

// ── Multi-line wrap (clamping) ──

#[test]
fn explicit_width_constrains_text_box() {
    let fp = MockFontProvider::new();
    let doc = run(
        r#"<div style="position: absolute; display: flex"><span id="s" style="font-size: 20px; width: 30px">abcde</span></div>"#,
        &[],
        &fp,
    );
    // measure returns 60 width, but explicit width:30px caps the box.
    let span = find_by_id(&doc, "s");
    let bx = &doc.get(span).unwrap().box_pixel;
    assert!(close(bx.width, 30.0, 1.0), "constrained width = {}", bx.width);
}
