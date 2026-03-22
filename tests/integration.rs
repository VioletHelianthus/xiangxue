use xiangxue::{
    parse_html, resolve_layout, Dimension, Orientation, WidgetKind,
};

#[test]
fn full_page_structure() {
    let html = r#"
    <html>
    <head><title>Test</title></head>
    <body>
        <div data-name="root" style="display:flex;flex-direction:column">
            <div data-name="header" style="display:flex;flex-direction:row">
                <h1 id="title">Title</h1>
                <div data-widget="Button" id="close_btn">X</div>
            </div>
            <div data-name="body" style="overflow:scroll">
                <p data-name="content">Hello</p>
            </div>
        </div>
    </body>
    </html>
    "#;

    let tree = parse_html(html);
    assert_eq!(tree.name, "root");
    assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));

    assert_eq!(tree.children.len(), 2);

    let header = &tree.children[0];
    assert_eq!(header.name, "header");
    assert_eq!(header.widget, WidgetKind::Layout(Orientation::Horizontal));
    assert_eq!(header.children.len(), 2);

    let title = &header.children[0];
    assert_eq!(title.name, "title");
    assert_eq!(title.widget, WidgetKind::Text);

    let close_btn = &header.children[1];
    assert_eq!(close_btn.name, "close_btn");
    assert_eq!(close_btn.widget, WidgetKind::Button);

    let body = &tree.children[1];
    assert_eq!(body.name, "body");
    assert_eq!(body.widget, WidgetKind::ScrollView);
}

#[test]
fn format_tree_output() {
    let html = r#"
    <div data-name="root" style="display:flex;flex-direction:column">
        <div data-widget="Button" id="ok">OK</div>
        <img data-name="icon" src="a.png"/>
    </div>
    "#;

    let tree = parse_html(html);
    let output = xiangxue::format_tree(&tree);

    assert!(output.contains("root: Layout(Vertical)"));
    assert!(output.contains("  ok: Button {text: \"OK\"}"));
    assert!(output.contains("  icon: Image"));
}

#[test]
fn mixed_content_container() {
    let html = r#"<div data-name="box">Some text<div data-widget="Button">Click</div>More text</div>"#;
    let tree = parse_html(html);

    assert_eq!(tree.name, "box");
    assert_eq!(tree.children.len(), 3);
    assert_eq!(tree.children[0].widget, WidgetKind::Text);
    assert_eq!(tree.children[1].widget, WidgetKind::Button);
    assert_eq!(tree.children[2].widget, WidgetKind::Text);
}

#[test]
fn semantic_tags_as_containers() {
    for tag in &["section", "header", "footer", "nav", "aside", "article", "main"] {
        let html = format!("<{tag} data-name=\"c\"><p>x</p></{tag}>");
        let tree = parse_html(&html);
        assert_eq!(
            tree.widget,
            WidgetKind::Layout(Orientation::Vertical),
            "{tag} should be Layout(Vertical)"
        );
    }
}

#[test]
fn justify_content_center() {
    let html = r#"<div data-name="root" style="width:400px;height:100px;display:flex;flex-direction:row;justify-content:center">
        <div data-name="child" style="width:100px;height:50px"></div>
    </div>"#;

    let mut tree = parse_html(html);
    resolve_layout(&mut tree, 400.0, 100.0);

    let child = &tree.children[0];
    assert!((child.layout.resolved_x.unwrap() - 150.0).abs() < 0.1);
}

#[test]
fn data_star_attributes_collected() {
    let html = r#"<div data-widget="ProgressBar" data-name="hpBar"
                       data-value="850" data-max="1060"
                       style="width:200px;height:20px">
    </div>"#;

    let tree = parse_html(html);
    assert_eq!(tree.name, "hpBar");
    assert_eq!(tree.widget, WidgetKind::ProgressBar);
    assert_eq!(tree.attrs.get("data-value").map(|s| s.as_str()), Some("850"));
    assert_eq!(tree.attrs.get("data-max").map(|s| s.as_str()), Some("1060"));
}

#[test]
fn data_widget_with_arbitrary_data_attrs() {
    let html = r#"<div data-widget="Slider" data-name="volSlider"
                       data-min="0" data-max="100" data-step="5"
                       style="width:300px;height:30px">
    </div>"#;

    let tree = parse_html(html);
    assert_eq!(tree.widget, WidgetKind::Slider);
    assert_eq!(tree.attrs.get("data-min").map(|s| s.as_str()), Some("0"));
    assert_eq!(tree.attrs.get("data-max").map(|s| s.as_str()), Some("100"));
    assert_eq!(tree.attrs.get("data-step").map(|s| s.as_str()), Some("5"));
}
