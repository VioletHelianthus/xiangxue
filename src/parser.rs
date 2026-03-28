use std::collections::HashMap;

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::style::{has_overflow_scroll, parse_flex_direction, parse_layout_props};
use crate::types::{CssProperties, LayoutProps, Orientation, UiNode, WidgetKind};

/// Tags that should be skipped entirely during conversion.
const SKIP_TAGS: &[&str] = &["head", "script", "style", "link", "meta"];

/// Container-like tags that behave like `div` for classification purposes.
const CONTAINER_TAGS: &[&str] = &[
    "div", "section", "header", "footer", "nav", "aside", "article", "main", "form",
    // Tags that previously mapped to engine-specific controls now fall back to Layout.
    // Use data-widget or Vue components to get the correct engine type.
    "button", "a", "ul", "ol", "input", "textarea", "progress",
];

/// Leaf text tags.
const TEXT_TAGS: &[&str] = &["span", "p", "label", "h1", "h2", "h3", "h4", "h5", "h6"];

/// Parse an HTML string into a `UiNode` tree.
pub fn parse_html(html: &str) -> UiNode {
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("failed to parse HTML");

    // Find <body>, or fall back to the document root.
    let body = find_body(&dom.document).unwrap_or_else(|| dom.document.clone());

    let children = convert_children(&body, true);

    // If the body produced exactly one child, promote it to root.
    if children.len() == 1 {
        return children.into_iter().next().unwrap();
    }

    UiNode {
        name: "root".to_string(),
        widget: WidgetKind::Layout(Orientation::Vertical),
        children,
        attrs: HashMap::new(),
        css: CssProperties::default(),
        layout: LayoutProps::default(),
    }
}

/// Recursively search for the `<body>` element.
fn find_body(handle: &Handle) -> Option<Handle> {
    match &handle.data {
        NodeData::Element { name, .. } if name.local.as_ref() == "body" => {
            return Some(handle.clone());
        }
        _ => {}
    }
    for child in handle.children.borrow().iter() {
        if let Some(found) = find_body(child) {
            return Some(found);
        }
    }
    None
}

/// Get attribute value from an element's attribute list.
fn get_attr(attrs: &[html5ever::Attribute], key: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name.local.as_ref() == key)
        .map(|a| a.value.to_string())
}

/// Extract a name for the node: data-name > id > first class > tag name.
fn extract_name(tag: &str, attrs: &[html5ever::Attribute]) -> String {
    if let Some(name) = get_attr(attrs, "data-name") {
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(id) = get_attr(attrs, "id") {
        if !id.is_empty() {
            return id;
        }
    }
    if let Some(class) = get_attr(attrs, "class") {
        if let Some(first) = class.split_whitespace().next() {
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    tag.to_string()
}

/// Classify an HTML element into a `WidgetKind`.
fn classify_element(tag: &str, style: &str, _attrs: &[html5ever::Attribute]) -> WidgetKind {
    match tag {
        // Engine-agnostic mappings only: container, image, text.
        // Engine-specific controls (Button, Slider, CheckBox, etc.) require
        // data-widget or Vue components — HTML tags alone don't imply engine types.
        "img" => WidgetKind::Image,
        t if TEXT_TAGS.contains(&t) => WidgetKind::Text,
        t if CONTAINER_TAGS.contains(&t) => {
            if has_overflow_scroll(style) {
                WidgetKind::ScrollView
            } else if let Some(orientation) = parse_flex_direction(style) {
                WidgetKind::Layout(orientation)
            } else {
                WidgetKind::Layout(Orientation::Vertical)
            }
        }
        // Treat <li> as a vertical layout container so list items become children.
        "li" => WidgetKind::Layout(Orientation::Vertical),
        // Fallback
        other => WidgetKind::Unknown(other.to_string()),
    }
}

/// Parse a `data-widget` attribute value into a `WidgetKind`.
fn parse_data_widget(value: &str) -> WidgetKind {
    match value {
        "Layout" | "Panel" => WidgetKind::Layout(Orientation::Vertical),
        "HLayout" => WidgetKind::Layout(Orientation::Horizontal),
        "VLayout" => WidgetKind::Layout(Orientation::Vertical),
        "Button" => WidgetKind::Button,
        "Text" | "Label" => WidgetKind::Text,
        "Image" | "ImageView" => WidgetKind::Image,
        "ScrollView" => WidgetKind::ScrollView,
        "ListView" => WidgetKind::ListView,
        "TextField" => WidgetKind::TextField,
        "CheckBox" => WidgetKind::CheckBox,
        "Slider" => WidgetKind::Slider,
        "ProgressBar" | "LoadingBar" => WidgetKind::ProgressBar,
        // Cocos-specific types (via Unknown for extensibility)
        "TextBMFont" | "TextAtlas" | "Sprite" | "ProjectNode" | "Node" | "PageView"
        | "TabControl" => WidgetKind::Unknown(value.to_string()),
        other => WidgetKind::Unknown(other.to_string()),
    }
}

/// Returns true if this widget kind is a leaf that should collect text content.
fn is_leaf_widget(kind: &WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Button | WidgetKind::Text | WidgetKind::TextField
    ) || matches!(kind, WidgetKind::Unknown(name) if matches!(name.as_str(),
        "TextBMFont" | "TextAtlas" | "Sprite" | "ProjectNode"
    ))
}

/// Returns true if this widget kind is a container that should recurse children.
fn is_container_widget(kind: &WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Layout(_) | WidgetKind::ScrollView | WidgetKind::ListView
    )
}

/// Collect all descendant text content of a handle into a string.
fn collect_text(handle: &Handle) -> String {
    let mut result = String::new();
    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                result.push_str(&contents.borrow());
            }
            NodeData::Element { .. } => {
                result.push_str(&collect_text(child));
            }
            _ => {}
        }
    }
    result
}

/// Convert children of a handle into a list of `UiNode`s.
/// `is_container` determines whether bare text nodes become Text UiNodes.
fn convert_children(handle: &Handle, is_container: bool) -> Vec<UiNode> {
    let mut children = Vec::new();
    collect_children_flat(handle, is_container, &mut children);
    children
}

/// Recursively collect child nodes, handling `data-x-internal` nodes.
/// Internal wrapper nodes (with widget descendants) are kept as Layout nodes
/// so Taffy can resolve their CSS layout (e.g. grid context). The backend
/// skips emitting actors for them but uses their resolved layout for children.
/// Internal leaf nodes (img, span with only text) are dropped entirely.
fn collect_children_flat(handle: &Handle, is_container: bool, out: &mut Vec<UiNode>) {
    for child in handle.children.borrow().iter() {
        if is_internal_node(child) {
            if has_widget_descendants(child) {
                // Keep as a regular node so Taffy resolves its layout (grid, flex, etc.).
                // The data-x-internal attr is preserved so the backend can skip it.
                if let Some(node) = convert_node(child, true) {
                    out.push(node);
                }
            }
            // else: leaf internal node (img, span with text) — drop entirely
            continue;
        }
        if let Some(node) = convert_node(child, is_container) {
            out.push(node);
        }
    }
}

/// Check if a DOM node is marked as Vue-internal (data-x-internal attribute).
fn is_internal_node(handle: &Handle) -> bool {
    if let NodeData::Element { attrs, .. } = &handle.data {
        let attrs = attrs.borrow();
        get_attr(&attrs, "data-x-internal").is_some()
    } else {
        false
    }
}

/// Check if any descendant element has a data-widget attribute.
fn has_widget_descendants(handle: &Handle) -> bool {
    for child in handle.children.borrow().iter() {
        if let NodeData::Element { attrs, .. } = &child.data {
            let attrs = attrs.borrow();
            if get_attr(&attrs, "data-widget").is_some() {
                return true;
            }
        }
        if has_widget_descendants(child) {
            return true;
        }
    }
    false
}

/// Parse a `data-anchor` attribute value "x,y" into (f32, f32).
fn parse_data_anchor(value: &str) -> Option<(f32, f32)> {
    let (x, y) = value.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// Convert a single DOM node into an optional `UiNode`.
fn convert_node(handle: &Handle, parent_is_container: bool) -> Option<UiNode> {
    match &handle.data {
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.as_ref().to_lowercase();

            // Skip non-visual elements.
            if SKIP_TAGS.contains(&tag.as_str()) {
                return None;
            }
            // Skip <html> and <body> — just process their children as if transparent.
            // (They should not appear here because we start from body's children,
            //  but handle edge cases.)
            if tag == "html" || tag == "body" {
                // This shouldn't normally be reached, but if it is, flatten children.
                return None;
            }

            let attrs_borrowed = attrs.borrow();

            let style = get_attr(&attrs_borrowed, "style").unwrap_or_default();
            let has_data_widget = get_attr(&attrs_borrowed, "data-widget").is_some();
            let widget = if let Some(dw) = get_attr(&attrs_borrowed, "data-widget") {
                parse_data_widget(&dw)
            } else {
                classify_element(&tag, &style, &attrs_borrowed)
            };
            let node_name = extract_name(&tag, &attrs_borrowed);

            // Parse layout properties from inline style.
            let mut layout = parse_layout_props(&style);

            // data-anchor overrides transform-origin anchors.
            if let Some(anchor_str) = get_attr(&attrs_borrowed, "data-anchor") {
                if let Some((ax, ay)) = parse_data_anchor(&anchor_str) {
                    layout.anchor_x = Some(ax);
                    layout.anchor_y = Some(ay);
                }
            }

            // data-pivot preserves engine pivot for round-trip fidelity.
            if let Some(pivot_str) = get_attr(&attrs_borrowed, "data-pivot") {
                if let Some((px, py)) = parse_data_anchor(&pivot_str) {
                    layout.pivot_x = Some(px);
                    layout.pivot_y = Some(py);
                }
            }

            let mut node_attrs = HashMap::new();

            // Copy relevant HTML attributes.
            for attr in attrs_borrowed.iter() {
                let aname = attr.name.local.as_ref().to_string();
                match aname.as_str() {
                    "src" | "href" | "alt" | "placeholder" | "type" | "value" => {
                        node_attrs.insert(aname, attr.value.to_string());
                    }
                    _ if aname.starts_with("data-") => {
                        node_attrs.insert(aname, attr.value.to_string());
                    }
                    _ => {}
                }
            }

            // Determine if this node is a leaf (no child recursion):
            // - Text-bearing widgets (Button, Text, TextField) always leaf
            // - data-widget on a non-container widget is atomic: SSR child
            //   nodes are visual implementation details, not user intent
            let is_atomic = is_leaf_widget(&widget)
                || (has_data_widget && !is_container_widget(&widget));

            if is_atomic {
                // Leaf / atomic widget: collect text content, skip children.
                let text = collect_text(handle).trim().to_string();
                if !text.is_empty() {
                    node_attrs.insert("text".to_string(), text);
                }
                Some(UiNode {
                    name: node_name,
                    widget,
                    children: Vec::new(),
                    attrs: node_attrs,
                    css: CssProperties { raw: style },
                    layout,
                })
            } else {
                // Container widget: recurse into children.
                let children = convert_children(handle, true);
                Some(UiNode {
                    name: node_name,
                    widget,
                    children,
                    attrs: node_attrs,
                    css: CssProperties { raw: style },
                    layout,
                })
            }
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().trim().to_string();
            if text.is_empty() {
                return None;
            }
            if parent_is_container {
                // Bare text in a container becomes a Text node.
                let mut attrs = HashMap::new();
                attrs.insert("text".to_string(), text.clone());
                Some(UiNode {
                    name: "text".to_string(),
                    widget: WidgetKind::Text,
                    children: Vec::new(),
                    attrs,
                    css: CssProperties::default(),
                    layout: LayoutProps::default(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_button_becomes_layout() {
        // Without data-widget, <button> is just a container
        let tree = parse_html("<button id=\"ok\">OK</button>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.name, "ok");
    }

    #[test]
    fn data_widget_button() {
        let tree = parse_html("<div data-widget=\"Button\" id=\"ok\">OK</div>");
        assert_eq!(tree.widget, WidgetKind::Button);
        assert_eq!(tree.name, "ok");
        assert_eq!(tree.attrs.get("text").map(|s| s.as_str()), Some("OK"));
    }

    #[test]
    fn div_defaults_to_vertical_layout() {
        let tree = parse_html("<div data-name=\"root\"><p>hello</p></div>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.name, "root");
    }

    #[test]
    fn flex_row_becomes_horizontal() {
        let tree =
            parse_html("<div style=\"display:flex;flex-direction:row\"><p>a</p><p>b</p></div>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Horizontal));
    }

    #[test]
    fn overflow_scroll_becomes_scrollview() {
        let tree = parse_html("<div style=\"overflow:scroll\"><p>a</p></div>");
        assert_eq!(tree.widget, WidgetKind::ScrollView);
    }

    #[test]
    fn img_element() {
        let tree = parse_html("<img data-name=\"icon\" src=\"a.png\"/>");
        assert_eq!(tree.widget, WidgetKind::Image);
        assert_eq!(tree.name, "icon");
        assert_eq!(tree.attrs.get("src").map(|s| s.as_str()), Some("a.png"));
    }

    #[test]
    fn name_from_class() {
        let tree = parse_html("<div class=\"sidebar main\"><p>x</p></div>");
        assert_eq!(tree.name, "sidebar");
    }

    #[test]
    fn text_tags() {
        for tag in &["span", "p", "label", "h1", "h2", "h3"] {
            let html = format!("<{tag}>hello</{tag}>");
            let tree = parse_html(&html);
            assert_eq!(tree.widget, WidgetKind::Text, "tag {tag} should be Text");
        }
    }

    #[test]
    fn bare_list_becomes_layout() {
        // Without data-widget, <ul> is just a container
        let tree = parse_html("<ul><li>A</li><li>B</li></ul>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn data_widget_listview() {
        let tree = parse_html("<div data-widget=\"ListView\"><li>A</li><li>B</li></div>");
        assert_eq!(tree.widget, WidgetKind::ListView);
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn bare_input_becomes_layout() {
        // Without data-widget, <input> is just a container
        let tree = parse_html("<input placeholder=\"type here\"/>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(
            tree.attrs.get("placeholder").map(|s| s.as_str()),
            Some("type here")
        );
    }

    #[test]
    fn bare_checkbox_becomes_layout() {
        let tree = parse_html("<input type=\"checkbox\" data-name=\"toggle\"/>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.name, "toggle");
    }

    #[test]
    fn bare_range_becomes_layout() {
        let tree = parse_html("<input type=\"range\" data-name=\"vol\"/>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
    }

    #[test]
    fn bare_progress_becomes_layout() {
        let tree = parse_html("<progress data-name=\"hp\"></progress>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.name, "hp");
    }

    #[test]
    fn data_widget_controls() {
        // Engine controls require data-widget
        let cases = vec![
            ("TextField", WidgetKind::TextField),
            ("CheckBox", WidgetKind::CheckBox),
            ("Slider", WidgetKind::Slider),
            ("ProgressBar", WidgetKind::ProgressBar),
        ];
        for (widget_name, expected) in cases {
            let html = format!("<div data-widget=\"{widget_name}\" data-name=\"test\"></div>");
            let tree = parse_html(&html);
            assert_eq!(tree.widget, expected, "data-widget={widget_name}");
        }
    }

    #[test]
    fn data_widget_pageview() {
        let tree = parse_html("<div data-widget=\"PageView\" data-name=\"pages\"></div>");
        assert_eq!(tree.widget, WidgetKind::Unknown("PageView".to_string()));
        assert_eq!(tree.name, "pages");
    }

    #[test]
    fn data_widget_overrides_tag() {
        // A <div> with data-widget="Button" should become Button, not Layout
        let tree = parse_html("<div data-widget=\"Button\" data-name=\"btn\">Click</div>");
        assert_eq!(tree.widget, WidgetKind::Button);
        assert_eq!(tree.name, "btn");
    }

    #[test]
    fn data_widget_checkbox() {
        let tree = parse_html("<span data-widget=\"CheckBox\" data-name=\"cb\">x</span>");
        assert_eq!(tree.widget, WidgetKind::CheckBox);
    }

    #[test]
    fn bare_text_in_container() {
        let tree = parse_html("<div data-name=\"box\">Hello world</div>");
        assert_eq!(tree.widget, WidgetKind::Layout(Orientation::Vertical));
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].widget, WidgetKind::Text);
        assert_eq!(
            tree.children[0].attrs.get("text").map(|s| s.as_str()),
            Some("Hello world")
        );
    }

    #[test]
    fn data_anchor_overrides_transform_origin() {
        let tree = parse_html(
            r#"<img data-name="icon" src="a.png" style="transform-origin: center" data-anchor="0,0"/>"#,
        );
        assert_eq!(tree.layout.anchor_x, Some(0.0));
        assert_eq!(tree.layout.anchor_y, Some(0.0));
    }

    #[test]
    fn data_anchor_parsed_to_layout() {
        let tree = parse_html(r#"<img data-name="icon" src="a.png" data-anchor="0.3,0.7"/>"#);
        assert_eq!(tree.layout.anchor_x, Some(0.3));
        assert_eq!(tree.layout.anchor_y, Some(0.7));
    }

    #[test]
    fn css_width_propagated_to_layout() {
        let tree = parse_html(r#"<div data-widget="Button" id="ok" style="width:200px;height:60px">OK</div>"#);
        assert_eq!(
            tree.layout.width,
            Some(crate::types::Dimension::Px(200.0))
        );
        assert_eq!(tree.layout.height, Some(crate::types::Dimension::Px(60.0)));
    }

    #[test]
    fn skips_script_and_style() {
        let tree = parse_html(
            "<div data-name=\"root\"><script>alert(1)</script><style>body{}</style><p>hi</p></div>",
        );
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].widget, WidgetKind::Text);
    }

    #[test]
    fn full_document_with_head() {
        let tree = parse_html(
            "<html><head><title>T</title></head><body><div data-name=\"main\"><div data-widget=\"Button\">Go</div></div></body></html>",
        );
        assert_eq!(tree.name, "main");
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].widget, WidgetKind::Button);
    }
}
