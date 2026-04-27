//! CSS grid cascade + style_to_taffy mapping tests.

use xiangxue::{
    GridAutoFlow, GridLine, GridRepeatCount, GridTemplateComponent, GridTrackSize, NodeKind,
};

fn build(html: &str, css: &[&str]) -> xiangxue::Document {
    let mut doc = xiangxue::parse::html::parse(html).expect("html parses");
    xiangxue::cascade::run(&mut doc, &[], css).expect("cascade succeeds");
    doc
}

fn find_by_id<'a>(doc: &'a xiangxue::Document, want: &str) -> &'a xiangxue::NodeData {
    fn walk<'b>(
        doc: &'b xiangxue::Document,
        id: xiangxue::NodeId,
        want: &str,
        out: &mut Option<&'b xiangxue::NodeData>,
    ) {
        let Some(n) = doc.get(id) else { return };
        if let NodeKind::Element { attrs, .. } = &n.kind
            && attrs.get("id").map(|s| s.as_str()) == Some(want)
            && out.is_none()
        {
            *out = Some(n);
        }
        for &c in &n.children {
            walk(doc, c, want, out);
        }
    }
    let mut out = None;
    walk(doc, doc.root, want, &mut out);
    out.unwrap_or_else(|| panic!("no #{want} in document"))
}

fn computed<'a>(node: &'a xiangxue::NodeData) -> &'a xiangxue::ComputedStyle {
    match &node.kind {
        NodeKind::Element { computed, .. } => computed,
        _ => panic!("not an element"),
    }
}

#[test]
fn grid_template_columns_three_fixed_tracks() {
    let doc = build(
        r#"<body><div id="g" style="display: grid; grid-template-columns: 100px 200px 50px"></div></body>"#,
        &[],
    );
    let node = find_by_id(&doc, "g");
    let cs = computed(node);
    let grid = cs.grid.as_ref().expect("grid props populated");
    assert_eq!(grid.template_columns.len(), 3);
    let widths: Vec<_> = grid
        .template_columns
        .iter()
        .map(|c| match c {
            GridTemplateComponent::Single(track) => track.min.clone(),
            _ => panic!("expected single track"),
        })
        .collect();
    assert!(matches!(widths[0], GridTrackSize::Px(v) if (v - 100.0).abs() < 0.001));
    assert!(matches!(widths[1], GridTrackSize::Px(v) if (v - 200.0).abs() < 0.001));
    assert!(matches!(widths[2], GridTrackSize::Px(v) if (v - 50.0).abs() < 0.001));
}

#[test]
fn grid_template_columns_repeat() {
    let doc = build(
        r#"<body><div id="g" style="display: grid; grid-template-columns: repeat(3, 1fr)"></div></body>"#,
        &[],
    );
    let node = find_by_id(&doc, "g");
    let grid = computed(node).grid.as_ref().expect("grid populated");
    assert_eq!(grid.template_columns.len(), 1);
    match &grid.template_columns[0] {
        GridTemplateComponent::Repeat { count, tracks } => {
            assert_eq!(*count, GridRepeatCount::Count(3));
            assert_eq!(tracks.len(), 1);
            assert!(matches!(tracks[0].max, GridTrackSize::Fr(v) if (v - 1.0).abs() < 0.001));
        }
        _ => panic!("expected repeat"),
    }
}

#[test]
fn grid_auto_flow_column() {
    let doc = build(
        r#"<body><div id="g" style="display: grid; grid-auto-flow: column"></div></body>"#,
        &[],
    );
    let grid = computed(find_by_id(&doc, "g"))
        .grid
        .as_ref()
        .expect("grid populated");
    assert_eq!(grid.auto_flow, GridAutoFlow::Column);
}

#[test]
fn grid_template_areas_parses() {
    let doc = build(
        r#"<body><div id="g" style='display: grid; grid-template-areas: "a a b" "c d b"'></div></body>"#,
        &[],
    );
    let grid = computed(find_by_id(&doc, "g"))
        .grid
        .as_ref()
        .expect("grid populated");
    let areas = grid.template_areas.as_ref().expect("template areas set");
    assert_eq!(areas.columns, 3);
    assert_eq!(areas.areas.len(), 6);
}

#[test]
fn grid_explicit_placement_lines() {
    let doc = build(
        r#"<body><div id="item" style="grid-column: 2 / 4; grid-row: 1 / 3"></div></body>"#,
        &[],
    );
    let cs = computed(find_by_id(&doc, "item"));
    assert!(matches!(cs.grid_column.0, GridLine::Index(2)));
    assert!(matches!(cs.grid_column.1, GridLine::Index(4)));
    assert!(matches!(cs.grid_row.0, GridLine::Index(1)));
    assert!(matches!(cs.grid_row.1, GridLine::Index(3)));
}

#[test]
fn grid_area_named() {
    let doc = build(
        r#"<body><div id="item" style="grid-area: header"></div></body>"#,
        &[],
    );
    let cs = computed(find_by_id(&doc, "item"));
    match &cs.grid_column.0 {
        GridLine::Named(s) => assert_eq!(s, "header"),
        other => panic!("unexpected start: {other:?}"),
    }
    match &cs.grid_row.0 {
        GridLine::Named(s) => assert_eq!(s, "header"),
        other => panic!("unexpected start: {other:?}"),
    }
}

#[test]
fn grid_template_columns_flushes_to_taffy_style() {
    let mut doc = build(
        r#"<body><div id="g" style="display: grid; grid-template-columns: 100px 1fr 50px"></div></body>"#,
        &[],
    );
    xiangxue::layout::flush_styles(&mut doc).expect("flush ok");
    let id = find_by_id(&doc, "g").id;
    let n = doc.get(id).unwrap();
    assert_eq!(n.taffy_style.grid_template_columns.len(), 3);
}
