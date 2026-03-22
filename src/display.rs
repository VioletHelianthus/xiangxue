use crate::types::{Orientation, UiNode, WidgetKind};

/// Format the UiNode tree as an indented string for display.
pub fn format_tree(node: &UiNode) -> String {
    let mut output = String::new();
    format_node(node, 0, &mut output);
    output
}

fn format_node(node: &UiNode, indent: usize, output: &mut String) {
    let prefix = "  ".repeat(indent);
    let kind = format_widget(&node.widget);
    let text_suffix = if let Some(text) = node.attrs.get("text") {
        format!(" {{text: {:?}}}", text)
    } else {
        String::new()
    };

    output.push_str(&format!("{}{}: {}{}\n", prefix, node.name, kind, text_suffix));

    for child in &node.children {
        format_node(child, indent + 1, output);
    }
}

fn format_widget(widget: &WidgetKind) -> &'static str {
    match widget {
        WidgetKind::Layout(Orientation::Horizontal) => "Layout(Horizontal)",
        WidgetKind::Layout(Orientation::Vertical) => "Layout(Vertical)",
        WidgetKind::Button => "Button",
        WidgetKind::Text => "Text",
        WidgetKind::Image => "Image",
        WidgetKind::ScrollView => "ScrollView",
        WidgetKind::ListView => "ListView",
        WidgetKind::TextField => "TextField",
        WidgetKind::CheckBox => "CheckBox",
        WidgetKind::Slider => "Slider",
        WidgetKind::ProgressBar => "ProgressBar",
        WidgetKind::Unknown(_) => "Unknown",
    }
}
