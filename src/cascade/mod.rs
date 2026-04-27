//! CSS cascade pipeline.
//!
//! Inputs: a parsed [`Document`], external CSS strings, and inline style
//! attributes carried on Element nodes (`style="..."`).
//!
//! Outputs: each Element's `kind.computed: ComputedStyle` is filled following
//! W3C cascade order (origin & importance → specificity → source order),
//! followed by inheritance for inheritable properties.
//!
//! Scope is bounded by `docs/xiangxue-css-subset.md` (see §1 selectors,
//! §2-§13 properties). Anything outside the subset returns
//! `LayoutError::UnsupportedCss` instead of being silently ignored.

mod inherit;
mod properties;
mod selectors;

use lightningcss::declaration::DeclarationBlock;
use lightningcss::properties::Property;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use static_self::IntoOwned;

use crate::document::Document;
use crate::error::LayoutError;
use crate::node::{NodeData, NodeId, NodeKind};
use crate::style::ComputedStyle;

use self::selectors::MatchSelector;

/// Minimal UA stylesheet. Browsers ship hundreds of rules here; we ship
/// exactly what the layout subset needs.
const UA_STYLESHEET: &str = "
html, body { width: 100%; height: 100% }
";

/// Run cascade + inheritance for every Element in `doc`.
pub fn run(
    doc: &mut Document,
    stylesheets: &[String],
    extra_css: &[&str],
) -> Result<(), LayoutError> {
    // Combine all CSS sources. The UA stylesheet goes first (lowest priority)
    // so author rules can override; it just primes html/body to fill viewport
    // — without it, % sizing on top-level descendants resolves to 0 since
    // <html>/<body> default to auto height.
    let mut all_css: Vec<String> = Vec::new();
    all_css.push(UA_STYLESHEET.to_string());
    all_css.extend(stylesheets.iter().cloned());
    all_css.extend(extra_css.iter().map(|s| (*s).to_string()));
    collect_inline_stylesheets(doc, &mut all_css);

    // Parse each stylesheet and walk its rules immediately. `walk_rules` calls
    // `into_owned` on each Property (lightningcss `into_owned` feature) so the
    // CompiledRule outlives the parsed StyleSheet — the borrowed `&str` source
    // can be dropped after this loop finishes.
    let mut compiled: Vec<CompiledRule> = Vec::new();
    let mut order_counter: u32 = 0;
    for css in &all_css {
        let sheet = StyleSheet::parse(css.as_str(), ParserOptions::default())
            .map_err(|e| LayoutError::CssParse(e.to_string()))?;
        walk_rules(&sheet.rules.0, &mut compiled, &mut order_counter)?;
    }

    // Topological order so children read finalised parent style for inheritance.
    let order: Vec<NodeId> = topological_order(doc);
    for node_id in order {
        let is_element = doc
            .get(node_id)
            .map(|n| matches!(n.kind, NodeKind::Element { .. }))
            .unwrap_or(false);
        if !is_element {
            continue;
        }

        let mut new_style = ComputedStyle::initial();

        // Inherit from parent first.
        let parent_style: Option<ComputedStyle> = doc
            .get(node_id)
            .and_then(|n| n.parent)
            .and_then(|pid| doc.get(pid))
            .and_then(|pn| match &pn.kind {
                NodeKind::Element { computed, .. } => Some(computed.clone()),
                _ => None,
            });
        if let Some(p) = &parent_style {
            inherit::inherit_from(&mut new_style, p);
        }

        // Cascade matched rules (lower specificity first; later wins).
        let mut hits: Vec<&CompiledRule> = compiled
            .iter()
            .filter(|r| r.selector.matches(doc, node_id))
            .collect();
        hits.sort_by_key(|r| (r.selector.specificity(), r.source_order));

        for rule in hits {
            for prop in &rule.normal {
                properties::apply_property(&mut new_style, prop)?;
            }
            for prop in &rule.important {
                properties::apply_property(&mut new_style, prop)?;
            }
        }

        // Inline `style="..."` wins over external rules (highest specificity).
        let inline_css: Option<String> = doc.get(node_id).and_then(|n| match &n.kind {
            NodeKind::Element { attrs, .. } => attrs.get("style").cloned(),
            _ => None,
        });
        if let Some(inline) = inline_css {
            apply_inline_style(&inline, &mut new_style)?;
        }

        if let Some(node) = doc.get_mut(node_id) {
            if let NodeKind::Element { computed, .. } = &mut node.kind {
                *computed = new_style;
            }
        }
    }

    Ok(())
}

/// One compiled rule with owned property data.
struct CompiledRule {
    selector: MatchSelector,
    normal: Vec<Property<'static>>,
    important: Vec<Property<'static>>,
    source_order: u32,
}

fn walk_rules<'i>(
    rules: &[CssRule<'i>],
    out: &mut Vec<CompiledRule>,
    order_counter: &mut u32,
) -> Result<(), LayoutError> {
    for rule in rules {
        match rule {
            CssRule::Style(style_rule) => {
                let normal: Vec<Property<'static>> = style_rule
                    .declarations
                    .declarations
                    .iter()
                    .map(|p| p.clone().into_owned())
                    .collect();
                let important: Vec<Property<'static>> = style_rule
                    .declarations
                    .important_declarations
                    .iter()
                    .map(|p| p.clone().into_owned())
                    .collect();
                for sel in style_rule.selectors.0.iter() {
                    let compiled = selectors::compile(sel)?;
                    out.push(CompiledRule {
                        selector: compiled,
                        normal: normal.clone(),
                        important: important.clone(),
                        source_order: *order_counter,
                    });
                    *order_counter += 1;
                }
            }
            CssRule::Media(_)
            | CssRule::Supports(_)
            | CssRule::FontFace(_)
            | CssRule::Keyframes(_)
            | CssRule::Page(_)
            | CssRule::Container(_) => {
                return Err(LayoutError::UnsupportedCss {
                    feature: format!("@-rule: {}", rule_name(rule)),
                    location: None,
                });
            }
            CssRule::Import(_)
            | CssRule::Namespace(_)
            | CssRule::LayerStatement(_)
            | CssRule::Property(_) => {
                // Silently ignore — these don't contribute matchable rules.
            }
            CssRule::LayerBlock(layer) => walk_rules(&layer.rules.0, out, order_counter)?,
            _ => {
                return Err(LayoutError::UnsupportedCss {
                    feature: "unknown CSS rule".into(),
                    location: None,
                });
            }
        }
    }
    Ok(())
}

fn rule_name(rule: &CssRule<'_>) -> &'static str {
    match rule {
        CssRule::Media(_) => "@media",
        CssRule::Supports(_) => "@supports",
        CssRule::FontFace(_) => "@font-face",
        CssRule::Keyframes(_) => "@keyframes",
        CssRule::Page(_) => "@page",
        CssRule::Container(_) => "@container",
        _ => "unknown",
    }
}

fn apply_inline_style(css: &str, style: &mut ComputedStyle) -> Result<(), LayoutError> {
    let block = DeclarationBlock::parse_string(css, ParserOptions::default())
        .map_err(|e| LayoutError::CssParse(format!("inline style: {e}")))?;
    for prop in &block.declarations {
        properties::apply_property(style, prop)?;
    }
    for prop in &block.important_declarations {
        properties::apply_property(style, prop)?;
    }
    Ok(())
}

/// Pre-order traversal IDs so each child is processed after its parent.
fn topological_order(doc: &Document) -> Vec<NodeId> {
    let mut order = Vec::new();
    let mut stack = vec![doc.root];
    while let Some(id) = stack.pop() {
        order.push(id);
        if let Some(node) = doc.get(id) {
            for &c in node.children.iter().rev() {
                stack.push(c);
            }
        }
    }
    order
}

fn collect_inline_stylesheets(doc: &Document, sink: &mut Vec<String>) {
    fn walk(doc: &Document, id: NodeId, sink: &mut Vec<String>) {
        if let Some(node) = doc.get(id) {
            if let NodeKind::Element { tag, .. } = &node.kind {
                if tag.eq_ignore_ascii_case("style") {
                    let mut text = String::new();
                    collect_text(doc, node, &mut text);
                    if !text.trim().is_empty() {
                        sink.push(text);
                    }
                }
            }
            for &c in &node.children {
                walk(doc, c, sink);
            }
        }
    }
    walk(doc, doc.root, sink);
}

fn collect_text(doc: &Document, node: &NodeData, out: &mut String) {
    if let NodeKind::Text(s) = &node.kind {
        out.push_str(s);
    }
    for &c in &node.children {
        if let Some(n) = doc.get(c) {
            collect_text(doc, n, out);
        }
    }
}
