//! Layout phase via Taffy.
//!
//! Goes through the full pipeline (parse → cascade → flush → solve) and
//! checks `box_pixel` results match expected positions.

use xiangxue::{BoxModel, NodeKind};

fn pipeline(html: &str, css: &[&str]) -> xiangxue::Document {
    let mut doc = xiangxue::parse::html::parse(html).unwrap();
    xiangxue::cascade::run(&mut doc, &[], css).unwrap();
    xiangxue::layout::flush_styles(&mut doc).unwrap();
    let opts = xiangxue::LayoutOptions {
        viewport: xiangxue::Size::new(800.0, 600.0),
        ..Default::default()
    };
    xiangxue::layout::solve(
        &mut doc,
        &opts,
        &xiangxue::NoOpFontProvider,
    )
    .unwrap();
    doc
}

fn find_by_id(doc: &xiangxue::Document, id_attr: &str) -> xiangxue::NodeId {
    fn walk(doc: &xiangxue::Document, id: xiangxue::NodeId, want: &str, out: &mut Option<xiangxue::NodeId>) {
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

fn pixel<'a>(doc: &'a xiangxue::Document, id: xiangxue::NodeId) -> &'a BoxModel {
    &doc.get(id).unwrap().box_pixel
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0
}

// ── Box model basics ──

#[test]
fn fixed_size_block() {
    let doc = pipeline(
        r#"<div id="x" style="width: 200px; height: 100px"></div>"#,
        &[],
    );
    let bx = pixel(&doc, find_by_id(&doc, "x"));
    assert!(close(bx.width, 200.0), "width was {}", bx.width);
    assert!(close(bx.height, 100.0), "height was {}", bx.height);
}

#[test]
fn padding_applies_to_layout() {
    // A child fills its parent's content area minus padding.
    let doc = pipeline(
        r#"<div id="parent" style="width: 400px; height: 200px; padding: 20px; display: flex">
             <div id="child" style="flex-grow: 1; height: 100%"></div>
           </div>"#,
        &[],
    );
    let parent = pixel(&doc, find_by_id(&doc, "parent"));
    let child = pixel(&doc, find_by_id(&doc, "child"));
    assert!(close(parent.width, 400.0));
    // Child is positioned at padding offset relative to parent.
    assert!(close(child.x, 20.0), "child.x was {}", child.x);
    assert!(close(child.y, 20.0), "child.y was {}", child.y);
    // Child width = parent content width = 400 - 40 = 360
    assert!(close(child.width, 360.0), "child.width was {}", child.width);
}

#[test]
fn percentage_sizing() {
    let doc = pipeline(
        r#"<div id="x" style="width: 50%; height: 25%"></div>"#,
        &[],
    );
    let bx = pixel(&doc, find_by_id(&doc, "x"));
    // viewport is 800×600 → 50% × 25% = 400×150
    assert!(close(bx.width, 400.0), "width was {}", bx.width);
    assert!(close(bx.height, 150.0), "height was {}", bx.height);
}

// ── Flex ──

#[test]
fn flex_row_distributes_children() {
    let doc = pipeline(
        r#"<div id="row" style="display: flex; width: 600px; height: 100px">
             <div id="a" style="width: 200px; height: 100px"></div>
             <div id="b" style="width: 100px; height: 100px"></div>
             <div id="c" style="width: 150px; height: 100px"></div>
           </div>"#,
        &[],
    );
    let a = pixel(&doc, find_by_id(&doc, "a"));
    let b = pixel(&doc, find_by_id(&doc, "b"));
    let c = pixel(&doc, find_by_id(&doc, "c"));
    assert!(close(a.x, 0.0));
    assert!(close(b.x, 200.0));
    assert!(close(c.x, 300.0));
}

#[test]
fn flex_column_stacks_children() {
    let doc = pipeline(
        r#"<div id="col" style="display: flex; flex-direction: column; width: 100px; height: 600px">
             <div id="a" style="width: 100px; height: 100px"></div>
             <div id="b" style="width: 100px; height: 200px"></div>
           </div>"#,
        &[],
    );
    let a = pixel(&doc, find_by_id(&doc, "a"));
    let b = pixel(&doc, find_by_id(&doc, "b"));
    assert!(close(a.y, 0.0));
    assert!(close(b.y, 100.0));
}

#[test]
fn flex_justify_center() {
    let doc = pipeline(
        r#"<div id="row" style="display: flex; justify-content: center; width: 600px; height: 100px">
             <div id="child" style="width: 100px; height: 100px"></div>
           </div>"#,
        &[],
    );
    let child = pixel(&doc, find_by_id(&doc, "child"));
    // 600 - 100 = 500 / 2 = 250
    assert!(close(child.x, 250.0), "child.x was {}", child.x);
}

#[test]
fn flex_grow_distributes_remaining() {
    let doc = pipeline(
        r#"<div id="row" style="display: flex; width: 600px; height: 100px">
             <div id="a" style="flex-grow: 1; height: 100px"></div>
             <div id="b" style="flex-grow: 2; height: 100px"></div>
           </div>"#,
        &[],
    );
    let a = pixel(&doc, find_by_id(&doc, "a"));
    let b = pixel(&doc, find_by_id(&doc, "b"));
    // a:b ratio = 1:2 → a=200, b=400
    assert!(close(a.width, 200.0), "a.width was {}", a.width);
    assert!(close(b.width, 400.0), "b.width was {}", b.width);
    assert!(close(b.x, 200.0));
}

#[test]
fn flex_align_items_center_vertically() {
    let doc = pipeline(
        r#"<div id="row" style="display: flex; align-items: center; width: 600px; height: 200px">
             <div id="child" style="width: 100px; height: 50px"></div>
           </div>"#,
        &[],
    );
    let child = pixel(&doc, find_by_id(&doc, "child"));
    // (200 - 50) / 2 = 75
    assert!(close(child.y, 75.0), "child.y was {}", child.y);
}

#[test]
fn flex_gap_separates_children() {
    let doc = pipeline(
        r#"<div id="row" style="display: flex; gap: 20px; width: 600px; height: 100px">
             <div id="a" style="width: 100px; height: 100px"></div>
             <div id="b" style="width: 100px; height: 100px"></div>
           </div>"#,
        &[],
    );
    let b = pixel(&doc, find_by_id(&doc, "b"));
    // a ends at 100, gap 20, b starts at 120
    assert!(close(b.x, 120.0), "b.x was {}", b.x);
}

// ── Position absolute ──

#[test]
fn absolute_inset_positions_relative_to_container() {
    let doc = pipeline(
        r#"<div id="container" style="position: relative; width: 400px; height: 300px">
             <div id="abs" style="position: absolute; left: 50px; top: 30px; width: 100px; height: 80px"></div>
           </div>"#,
        &[],
    );
    let abs = pixel(&doc, find_by_id(&doc, "abs"));
    assert!(close(abs.x, 50.0), "abs.x was {}", abs.x);
    assert!(close(abs.y, 30.0), "abs.y was {}", abs.y);
    assert!(close(abs.width, 100.0));
    assert!(close(abs.height, 80.0));
}

// ── Subpixel vs pixel rounding ──

#[test]
fn subpixel_and_pixel_layouts_both_set() {
    let doc = pipeline(
        r#"<div id="x" style="width: 100.5px; height: 50.5px"></div>"#,
        &[],
    );
    let id = find_by_id(&doc, "x");
    let node = doc.get(id).unwrap();
    // box_subpixel preserves fractional value
    assert!((node.box_subpixel.width - 100.5).abs() < 0.01);
    // box_pixel rounded to integer pixel grid
    assert_eq!(node.box_pixel.width.fract(), 0.0);
}

// ── Cascade-from-stylesheet feeds layout ──

#[test]
fn external_css_drives_layout() {
    let doc = pipeline(
        r#"<div id="x"></div>"#,
        &["#x { width: 320px; height: 64px }"],
    );
    let bx = pixel(&doc, find_by_id(&doc, "x"));
    assert!(close(bx.width, 320.0));
    assert!(close(bx.height, 64.0));
}

// ── End-to-end via xiangxue::layout one-shot entry ──

#[test]
fn one_shot_layout_entry_works() {
    let opts = xiangxue::LayoutOptions {
        viewport: xiangxue::Size::new(400.0, 300.0),
        ..Default::default()
    };
    let tree = xiangxue::layout(
        r#"<div id="x" style="width: 200px; height: 100px"></div>"#,
        &[],
        &opts,
    )
    .unwrap();
    let bx = &tree
        .document
        .get(find_by_id(&tree.document, "x"))
        .unwrap()
        .box_pixel;
    assert!(close(bx.width, 200.0));
    assert!(close(bx.height, 100.0));
}
