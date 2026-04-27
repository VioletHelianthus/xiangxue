//! CSS transform cascade tests.

use xiangxue::{Length, NodeKind, TransformOp};

fn build(html: &str, css: &[&str]) -> xiangxue::Document {
    let mut doc = xiangxue::parse::html::parse(html).expect("html parses");
    xiangxue::cascade::run(&mut doc, &[], css).expect("cascade succeeds");
    doc
}

fn try_build(html: &str) -> Result<xiangxue::Document, xiangxue::LayoutError> {
    let mut doc = xiangxue::parse::html::parse(html)?;
    xiangxue::cascade::run(&mut doc, &[], &[])?;
    Ok(doc)
}

fn find_by_id<'a>(doc: &'a xiangxue::Document, want: &str) -> &'a xiangxue::ComputedStyle {
    fn walk<'b>(
        doc: &'b xiangxue::Document,
        id: xiangxue::NodeId,
        want: &str,
        out: &mut Option<&'b xiangxue::ComputedStyle>,
    ) {
        let Some(n) = doc.get(id) else { return };
        if let NodeKind::Element { attrs, computed, .. } = &n.kind
            && attrs.get("id").map(|s| s.as_str()) == Some(want)
            && out.is_none()
        {
            *out = Some(computed);
        }
        for &c in &n.children {
            walk(doc, c, want, out);
        }
    }
    let mut out = None;
    walk(doc, doc.root, want, &mut out);
    out.unwrap_or_else(|| panic!("no #{want} in document"))
}

#[test]
fn translate_percent_both_axes() {
    let doc = build(
        r#"<body><div id="x" style="transform: translate(-50%, -50%)"></div></body>"#,
        &[],
    );
    let cs = find_by_id(&doc, "x");
    assert_eq!(cs.transforms.len(), 1);
    match &cs.transforms[0] {
        TransformOp::Translate { x, y } => {
            assert!(matches!(x, Length::Percent(p) if (p + 50.0).abs() < 0.001));
            assert!(matches!(y, Length::Percent(p) if (p + 50.0).abs() < 0.001));
        }
        other => panic!("expected translate, got {other:?}"),
    }
}

#[test]
fn translate_x_only() {
    let doc = build(
        r#"<body><div id="x" style="transform: translateX(-50%)"></div></body>"#,
        &[],
    );
    let cs = find_by_id(&doc, "x");
    assert_eq!(cs.transforms.len(), 1);
    match &cs.transforms[0] {
        TransformOp::Translate { x, y } => {
            assert!(matches!(x, Length::Percent(p) if (p + 50.0).abs() < 0.001));
            assert!(matches!(y, Length::Px(v) if v.abs() < 0.001));
        }
        other => panic!("expected translate, got {other:?}"),
    }
}

#[test]
fn rotate_and_scale_preserve_order() {
    let doc = build(
        r#"<body><div id="x" style="transform: rotate(45deg) scale(2)"></div></body>"#,
        &[],
    );
    let cs = find_by_id(&doc, "x");
    assert_eq!(cs.transforms.len(), 2);
    assert!(matches!(cs.transforms[0], TransformOp::Rotate(d) if (d - 45.0).abs() < 0.01));
    assert!(matches!(cs.transforms[1], TransformOp::Scale(x, y) if (x - 2.0).abs() < 0.001 && (y - 2.0).abs() < 0.001));
}

#[test]
fn matrix_is_unsupported() {
    let err = match try_build(
        r#"<body><div style="transform: matrix(1,0,0,1,0,0)"></div></body>"#,
    ) {
        Err(e) => e,
        Ok(_) => panic!("matrix should error"),
    };
    assert!(matches!(err, xiangxue::LayoutError::UnsupportedCss { .. }));
}

#[test]
fn skew_is_unsupported() {
    let err = match try_build(
        r#"<body><div style="transform: skew(10deg)"></div></body>"#,
    ) {
        Err(e) => e,
        Ok(_) => panic!("skew should error"),
    };
    assert!(matches!(err, xiangxue::LayoutError::UnsupportedCss { .. }));
}
