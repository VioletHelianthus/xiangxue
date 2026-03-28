use taffy::prelude::{TaffyAuto, TaffyZero};
use taffy::{AvailableSpace, NodeId, Size, TaffyTree};

use crate::types::{Dimension, Orientation, UiNode};

/// Resolve flex layout for the entire UiNode tree using Taffy.
///
/// Writes `resolved_x/y/width/height` into each node's LayoutProps.
/// Coordinates are in CSS space (relative to parent, origin top-left, Y-down).
pub fn resolve_layout(root: &mut UiNode, design_width: f32, design_height: f32) {
    let mut taffy: TaffyTree = TaffyTree::new();

    let root_id = build_taffy_node(&mut taffy, root);

    taffy
        .compute_layout(
            root_id,
            Size {
                width: AvailableSpace::Definite(design_width),
                height: AvailableSpace::Definite(design_height),
            },
        )
        .expect("Taffy layout computation failed");

    write_back_layout(&taffy, root_id, root);
}

/// Recursively build a Taffy tree mirroring the UiNode tree.
fn build_taffy_node(taffy: &mut TaffyTree, node: &UiNode) -> NodeId {
    let style = to_taffy_style(&node.layout);

    if node.children.is_empty() {
        taffy.new_leaf(style).expect("Failed to create Taffy leaf")
    } else {
        let child_ids: Vec<NodeId> = node
            .children
            .iter()
            .map(|child| build_taffy_node(taffy, child))
            .collect();
        taffy
            .new_with_children(style, &child_ids)
            .expect("Failed to create Taffy node")
    }
}

/// Map LayoutProps to taffy::Style.
fn to_taffy_style(props: &crate::types::LayoutProps) -> taffy::Style {
    // If left/top are set but position is not declared, infer absolute
    // (backward compat: existing HTML uses left/top as direct coordinates).
    let position = match props.position {
        Some(p) => p,
        None => {
            if props.left.is_some() || props.top.is_some() || props.right.is_some() || props.bottom.is_some() {
                taffy::Position::Absolute
            } else {
                taffy::Position::Relative
            }
        }
    };

    let display = props.display.unwrap_or(taffy::Display::Flex);

    taffy::Style {
        display,
        position,
        flex_direction: match &props.flex_direction {
            Some(Orientation::Horizontal) => taffy::FlexDirection::Row,
            Some(Orientation::Vertical) | None => taffy::FlexDirection::Column,
        },
        flex_wrap: props.flex_wrap.unwrap_or(taffy::FlexWrap::NoWrap),
        justify_content: props.justify_content,
        align_items: props.align_items,
        align_self: props.align_self,
        justify_self: props.justify_self,
        flex_grow: props.flex_grow.unwrap_or(0.0),
        flex_shrink: props.flex_shrink.unwrap_or(1.0),
        flex_basis: props
            .flex_basis
            .as_ref()
            .map(dim_to_taffy)
            .unwrap_or(taffy::Dimension::AUTO),
        size: Size {
            width: props
                .width
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
            height: props
                .height
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
        },
        min_size: Size {
            width: props
                .min_width
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
            height: props
                .min_height
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
        },
        max_size: Size {
            width: props
                .max_width
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
            height: props
                .max_height
                .as_ref()
                .map(dim_to_taffy)
                .unwrap_or(taffy::Dimension::AUTO),
        },
        margin: taffy::Rect {
            top: opt_px_to_lpa(props.margin_top),
            right: opt_px_to_lpa(props.margin_right),
            bottom: opt_px_to_lpa(props.margin_bottom),
            left: opt_px_to_lpa(props.margin_left),
        },
        padding: taffy::Rect {
            top: opt_px_to_lp(props.padding_top),
            right: opt_px_to_lp(props.padding_right),
            bottom: opt_px_to_lp(props.padding_bottom),
            left: opt_px_to_lp(props.padding_left),
        },
        gap: Size {
            width: props.column_gap.or(props.gap)
                .map(taffy::LengthPercentage::length)
                .unwrap_or(taffy::LengthPercentage::ZERO),
            height: props.row_gap.or(props.gap)
                .map(taffy::LengthPercentage::length)
                .unwrap_or(taffy::LengthPercentage::ZERO),
        },
        // CSS Grid (container)
        grid_template_columns: props.grid_template_columns.clone(),
        grid_template_rows: props.grid_template_rows.clone(),
        grid_auto_flow: props.grid_auto_flow.unwrap_or(taffy::GridAutoFlow::Row),
        grid_auto_rows: props.grid_auto_rows.clone(),
        grid_auto_columns: props.grid_auto_columns.clone(),
        // CSS Grid (item)
        grid_column: taffy::Line {
            start: props.grid_column_start.clone().unwrap_or(taffy::GridPlacement::<String>::Auto),
            end: props.grid_column_end.clone().unwrap_or(taffy::GridPlacement::<String>::Auto),
        },
        grid_row: taffy::Line {
            start: props.grid_row_start.clone().unwrap_or(taffy::GridPlacement::<String>::Auto),
            end: props.grid_row_end.clone().unwrap_or(taffy::GridPlacement::<String>::Auto),
        },
        inset: taffy::Rect {
            left: props
                .left
                .as_ref()
                .map(dim_to_lpa)
                .unwrap_or(taffy::LengthPercentageAuto::AUTO),
            top: props
                .top
                .as_ref()
                .map(dim_to_lpa)
                .unwrap_or(taffy::LengthPercentageAuto::AUTO),
            right: props
                .right
                .as_ref()
                .map(dim_to_lpa)
                .unwrap_or(taffy::LengthPercentageAuto::AUTO),
            bottom: props
                .bottom
                .as_ref()
                .map(dim_to_lpa)
                .unwrap_or(taffy::LengthPercentageAuto::AUTO),
        },
        overflow: taffy::Point {
            x: if props.overflow_scroll {
                taffy::Overflow::Scroll
            } else {
                taffy::Overflow::Visible
            },
            y: if props.overflow_scroll {
                taffy::Overflow::Scroll
            } else {
                taffy::Overflow::Visible
            },
        },
        ..taffy::Style::DEFAULT
    }
}

/// Convert our Dimension (Percent is 0-100) to taffy::Dimension (percent is 0.0-1.0).
fn dim_to_taffy(dim: &Dimension) -> taffy::Dimension {
    match dim {
        Dimension::Px(v) => taffy::Dimension::length(*v),
        Dimension::Percent(pct) => taffy::Dimension::percent(*pct / 100.0),
    }
}

/// Convert our Dimension to taffy::LengthPercentageAuto.
fn dim_to_lpa(dim: &Dimension) -> taffy::LengthPercentageAuto {
    match dim {
        Dimension::Px(v) => taffy::LengthPercentageAuto::length(*v),
        Dimension::Percent(pct) => taffy::LengthPercentageAuto::percent(*pct / 100.0),
    }
}

/// Convert Option<f32> (px) to LengthPercentageAuto (0 if None).
fn opt_px_to_lpa(val: Option<f32>) -> taffy::LengthPercentageAuto {
    match val {
        Some(v) => taffy::LengthPercentageAuto::length(v),
        None => taffy::LengthPercentageAuto::length(0.0),
    }
}

/// Convert Option<f32> (px) to LengthPercentage (0 if None).
fn opt_px_to_lp(val: Option<f32>) -> taffy::LengthPercentage {
    match val {
        Some(v) => taffy::LengthPercentage::length(v),
        None => taffy::LengthPercentage::ZERO,
    }
}

/// Recursively write Taffy layout results back into UiNode.layout.resolved_* fields.
fn write_back_layout(taffy: &TaffyTree, node_id: NodeId, ui_node: &mut UiNode) {
    let layout = taffy.layout(node_id).expect("Failed to get layout");

    ui_node.layout.resolved_x = Some(layout.location.x);
    ui_node.layout.resolved_y = Some(layout.location.y);
    ui_node.layout.resolved_width = Some(layout.size.width);
    ui_node.layout.resolved_height = Some(layout.size.height);

    let child_ids = taffy.children(node_id).expect("Failed to get children");
    for (taffy_child, ui_child) in child_ids.iter().zip(ui_node.children.iter_mut()) {
        write_back_layout(taffy, *taffy_child, ui_child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_html;

    #[test]
    fn flex_row_positions_children_horizontally() {
        let html = r#"<div data-name="root" style="width:300px;height:100px;display:flex;flex-direction:row">
            <div data-name="a" style="width:100px;height:50px"></div>
            <div data-name="b" style="width:100px;height:50px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 300.0, 100.0);

        let a = &tree.children[0];
        let b = &tree.children[1];
        assert_eq!(a.layout.resolved_x, Some(0.0));
        assert!((b.layout.resolved_x.unwrap() - 100.0).abs() < 0.1);
        assert_eq!(a.layout.resolved_width, Some(100.0));
        assert_eq!(b.layout.resolved_width, Some(100.0));
    }

    #[test]
    fn justify_content_center() {
        let html = r#"<div data-name="root" style="width:300px;height:100px;display:flex;flex-direction:row;justify-content:center">
            <div data-name="child" style="width:100px;height:50px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 300.0, 100.0);

        let child = &tree.children[0];
        // Centered: (300 - 100) / 2 = 100
        assert!((child.layout.resolved_x.unwrap() - 100.0).abs() < 0.1);
    }

    #[test]
    fn flex_grow_distributes_space() {
        let html = r#"<div data-name="root" style="width:300px;height:100px;display:flex;flex-direction:row">
            <div data-name="a" style="flex-grow:1;height:50px"></div>
            <div data-name="b" style="flex-grow:2;height:50px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 300.0, 100.0);

        let a = &tree.children[0];
        let b = &tree.children[1];
        // a gets 1/3 = 100, b gets 2/3 = 200
        assert!((a.layout.resolved_width.unwrap() - 100.0).abs() < 0.1);
        assert!((b.layout.resolved_width.unwrap() - 200.0).abs() < 0.1);
    }

    #[test]
    fn absolute_position_with_left_top() {
        let html = r#"<div data-name="root" style="width:640px;height:960px">
            <div data-name="overlay" style="position:absolute;left:50px;top:100px;width:200px;height:150px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 640.0, 960.0);

        let overlay = &tree.children[0];
        assert!((overlay.layout.resolved_x.unwrap() - 50.0).abs() < 0.1);
        assert!((overlay.layout.resolved_y.unwrap() - 100.0).abs() < 0.1);
        assert!((overlay.layout.resolved_width.unwrap() - 200.0).abs() < 0.1);
        assert!((overlay.layout.resolved_height.unwrap() - 150.0).abs() < 0.1);
    }

    #[test]
    fn root_with_explicit_size() {
        let html = r#"<div data-name="root" style="width:640px;height:960px"></div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 640.0, 960.0);

        assert!((tree.layout.resolved_width.unwrap() - 640.0).abs() < 0.1);
        assert!((tree.layout.resolved_height.unwrap() - 960.0).abs() < 0.1);
    }

    #[test]
    fn nested_flex_layout() {
        let html = r#"<div data-name="root" style="width:400px;height:300px;display:flex;flex-direction:column">
            <div data-name="top" style="height:100px;display:flex;flex-direction:row">
                <div data-name="left" style="width:200px;height:100px"></div>
                <div data-name="right" style="width:200px;height:100px"></div>
            </div>
            <div data-name="bottom" style="height:200px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 400.0, 300.0);

        let top = &tree.children[0];
        let bottom = &tree.children[1];

        // top at y=0, bottom at y=100
        assert!((top.layout.resolved_y.unwrap()).abs() < 0.1);
        assert!((bottom.layout.resolved_y.unwrap() - 100.0).abs() < 0.1);

        // Inside top: left at x=0, right at x=200
        let left = &top.children[0];
        let right = &top.children[1];
        assert!((left.layout.resolved_x.unwrap()).abs() < 0.1);
        assert!((right.layout.resolved_x.unwrap() - 200.0).abs() < 0.1);
    }

    #[test]
    fn percent_size_resolved() {
        let html = r#"<div data-name="root" style="width:640px;height:960px">
            <div data-name="child" style="width:50%;height:25%"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 640.0, 960.0);

        let child = &tree.children[0];
        assert!((child.layout.resolved_width.unwrap() - 320.0).abs() < 0.1);
        assert!((child.layout.resolved_height.unwrap() - 240.0).abs() < 0.1);
    }

    #[test]
    fn grid_two_columns_equal() {
        let html = r#"<div data-name="root" style="width:400px;height:200px;display:grid;grid-template-columns:1fr 1fr;column-gap:0">
            <div data-name="a" style="height:50px"></div>
            <div data-name="b" style="height:50px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 400.0, 200.0);

        let a = &tree.children[0];
        let b = &tree.children[1];
        assert!((a.layout.resolved_width.unwrap() - 200.0).abs() < 0.1, "a width: {:?}", a.layout.resolved_width);
        assert!((b.layout.resolved_width.unwrap() - 200.0).abs() < 0.1, "b width: {:?}", b.layout.resolved_width);
        assert!((a.layout.resolved_x.unwrap()).abs() < 0.1);
        assert!((b.layout.resolved_x.unwrap() - 200.0).abs() < 0.1, "b x: {:?}", b.layout.resolved_x);
    }

    #[test]
    fn grid_fixed_and_fr_columns() {
        let html = r#"<div data-name="root" style="width:400px;height:200px;display:grid;grid-template-columns:100px 1fr 1fr">
            <div data-name="a"></div>
            <div data-name="b"></div>
            <div data-name="c"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 400.0, 200.0);

        let a = &tree.children[0];
        let b = &tree.children[1];
        let c = &tree.children[2];
        assert!((a.layout.resolved_width.unwrap() - 100.0).abs() < 0.1);
        assert!((b.layout.resolved_width.unwrap() - 150.0).abs() < 0.1);
        assert!((c.layout.resolved_width.unwrap() - 150.0).abs() < 0.1);
    }

    #[test]
    fn grid_with_gap() {
        let html = r#"<div data-name="root" style="width:420px;height:200px;display:grid;grid-template-columns:1fr 1fr;column-gap:20px">
            <div data-name="a" style="height:50px"></div>
            <div data-name="b" style="height:50px"></div>
        </div>"#;
        let mut tree = parse_html(html);
        resolve_layout(&mut tree, 420.0, 200.0);

        let a = &tree.children[0];
        let b = &tree.children[1];
        // (420 - 20 gap) / 2 = 200 each
        assert!((a.layout.resolved_width.unwrap() - 200.0).abs() < 0.1);
        assert!((b.layout.resolved_width.unwrap() - 200.0).abs() < 0.1);
        assert!((b.layout.resolved_x.unwrap() - 220.0).abs() < 0.1);
    }
}
