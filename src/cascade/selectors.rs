//! Selector compilation + matching for the v2 CSS subset (see
//! `docs/xiangxue-css-subset.md` §1).
//!
//! We do **not** implement `parcel_selectors::tree::Element` for our
//! `NodeData` — that trait surface is large and demands SelectorImpl plumbing.
//! Instead we walk lightningcss's already-parsed selector AST and build our
//! own [`MatchSelector`], rejecting anything outside the subset with
//! `LayoutError::UnsupportedCss`. Matching is then a tight ~150 lines of
//! tree-walking code that we fully own and test.

use lightningcss::selector::{Combinator, Selector};
use parcel_selectors::attr::AttrSelectorOperator;
use parcel_selectors::parser::{Component, NthType};

use crate::document::Document;
use crate::error::LayoutError;
use crate::node::{NodeData, NodeId, NodeKind};

/// One CSS selector compiled to our matcher's IR.
#[derive(Debug, Clone)]
pub struct MatchSelector {
    /// Compound selectors in **rightmost-first** order. `compounds[0]` is the
    /// target element's compound; later entries describe ancestors per the
    /// matching combinators.
    compounds: Vec<Compound>,
    /// Combinators between successive compounds. `len() = compounds.len() - 1`.
    /// `combinators[i]` is the relation between `compounds[i]` (descendant)
    /// and `compounds[i + 1]` (ancestor).
    combinators: Vec<MatchCombinator>,
    /// W3C specificity (a,b,c) packed in u32 by parcel_selectors.
    specificity: u32,
}

impl MatchSelector {
    pub fn specificity(&self) -> u32 {
        self.specificity
    }

    /// True if this selector matches the element with id `node_id` in `doc`.
    pub fn matches(&self, doc: &Document, node_id: NodeId) -> bool {
        if !self.compounds[0].matches(doc, node_id) {
            return false;
        }
        let mut current = match doc.get(node_id).and_then(|n| n.parent) {
            Some(p) => p,
            None => return self.combinators.is_empty(),
        };

        for (i, combinator) in self.combinators.iter().enumerate() {
            let target_compound = &self.compounds[i + 1];
            match combinator {
                MatchCombinator::Child => {
                    if !target_compound.matches(doc, current) {
                        return false;
                    }
                    current = match doc.get(current).and_then(|n| n.parent) {
                        Some(p) => p,
                        None => return i + 1 == self.combinators.len(),
                    };
                }
                MatchCombinator::Descendant => {
                    let mut ancestor = Some(current);
                    let matched_ancestor = loop {
                        let id = match ancestor {
                            Some(id) => id,
                            None => return false,
                        };
                        if target_compound.matches(doc, id) {
                            break id;
                        }
                        ancestor = doc.get(id).and_then(|n| n.parent);
                    };
                    current = match doc.get(matched_ancestor).and_then(|n| n.parent) {
                        Some(p) => p,
                        None => return i + 1 == self.combinators.len(),
                    };
                }
            }
        }
        true
    }
}

/// One compound selector (e.g. `div.foo#bar[data-x]:first-child`).
#[derive(Debug, Clone, Default)]
struct Compound {
    /// `None` when the compound starts with `*` (universal) or has no type
    /// selector; matches any element tag.
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<AttrMatch>,
    structural: Vec<StructuralPseudo>,
    /// If true, this compound never matches (selector containing a token
    /// guaranteed to fail).
    never_matches: bool,
}

impl Compound {
    fn matches(&self, doc: &Document, node_id: NodeId) -> bool {
        if self.never_matches {
            return false;
        }
        let node = match doc.get(node_id) {
            Some(n) => n,
            None => return false,
        };
        let (tag, attrs) = match &node.kind {
            NodeKind::Element { tag, attrs, .. } => (tag.as_str(), attrs),
            _ => return false,
        };

        if let Some(want_tag) = &self.tag {
            if !tag.eq_ignore_ascii_case(want_tag) {
                return false;
            }
        }

        if let Some(want_id) = &self.id {
            match attrs.get("id") {
                Some(id_val) if id_val == want_id => {}
                _ => return false,
            }
        }

        if !self.classes.is_empty() {
            let class_attr = attrs.get("class").map(String::as_str).unwrap_or("");
            for want in &self.classes {
                if !class_attr.split_ascii_whitespace().any(|c| c == want) {
                    return false;
                }
            }
        }

        for am in &self.attrs {
            let actual = attrs.get(&am.name);
            let ok = match (&am.op, actual) {
                (AttrOp::Exists, Some(_)) => true,
                (AttrOp::Equals(want), Some(have)) => have == want,
                _ => false,
            };
            if !ok {
                return false;
            }
        }

        for s in &self.structural {
            if !s.matches(doc, node, node_id) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
struct AttrMatch {
    name: String,
    op: AttrOp,
}

#[derive(Debug, Clone)]
enum AttrOp {
    Exists,
    Equals(String),
}

#[derive(Debug, Clone, Copy)]
enum MatchCombinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone)]
enum StructuralPseudo {
    FirstChild,
    LastChild,
    /// `:nth-child(an+b)`. For the simple form `:nth-child(n)` use a=0, b=n.
    NthChild { a: i32, b: i32 },
}

impl StructuralPseudo {
    fn matches(&self, doc: &Document, node: &NodeData, node_id: NodeId) -> bool {
        let parent = match node.parent.and_then(|p| doc.get(p)) {
            Some(p) => p,
            None => return false,
        };
        let element_siblings: Vec<NodeId> = parent
            .children
            .iter()
            .copied()
            .filter(|&c| {
                doc.get(c)
                    .map(|n| matches!(n.kind, NodeKind::Element { .. }))
                    .unwrap_or(false)
            })
            .collect();
        let index_0based = match element_siblings.iter().position(|&c| c == node_id) {
            Some(i) => i,
            None => return false,
        };
        let n = (index_0based + 1) as i32;
        match self {
            StructuralPseudo::FirstChild => n == 1,
            StructuralPseudo::LastChild => n == element_siblings.len() as i32,
            StructuralPseudo::NthChild { a, b } => {
                if *a == 0 {
                    n == *b
                } else {
                    let diff = n - b;
                    diff % a == 0 && diff / a >= 0
                }
            }
        }
    }
}

/// Compile a lightningcss selector to our IR. Rejects features outside
/// `xiangxue-css-subset.md` §1.
pub fn compile<'i>(selector: &Selector<'i>) -> Result<MatchSelector, LayoutError> {
    let specificity = selector.specificity();
    let mut compounds: Vec<Compound> = vec![Compound::default()];
    let mut combinators: Vec<MatchCombinator> = Vec::new();

    for component in selector.iter_raw_match_order() {
        match component {
            Component::Combinator(c) => match c {
                Combinator::Descendant => {
                    combinators.push(MatchCombinator::Descendant);
                    compounds.push(Compound::default());
                }
                Combinator::Child => {
                    combinators.push(MatchCombinator::Child);
                    compounds.push(Compound::default());
                }
                Combinator::NextSibling | Combinator::LaterSibling => {
                    return Err(unsupported("sibling combinator (+ / ~)"));
                }
                Combinator::PseudoElement | Combinator::SlotAssignment | Combinator::Part => {
                    return Err(unsupported("pseudo-element / slot / part combinator"));
                }
                _ => return Err(unsupported("unknown combinator")),
            },

            Component::ExplicitUniversalType
            | Component::ExplicitAnyNamespace
            | Component::ExplicitNoNamespace
            | Component::DefaultNamespace(_) => {
                // Universal / namespace tokens don't constrain the match.
            }

            Component::LocalName(ln) => {
                let name = ln.lower_name.0.as_ref().to_string();
                compounds.last_mut().unwrap().tag = Some(name);
            }

            Component::ID(ident) => {
                let name = ident.0.as_ref().to_string();
                compounds.last_mut().unwrap().id = Some(name);
            }

            Component::Class(ident) => {
                let name = ident.0.as_ref().to_string();
                compounds.last_mut().unwrap().classes.push(name);
            }

            Component::AttributeInNoNamespaceExists { local_name, .. } => {
                compounds.last_mut().unwrap().attrs.push(AttrMatch {
                    name: local_name.0.as_ref().to_string(),
                    op: AttrOp::Exists,
                });
            }
            Component::AttributeInNoNamespace {
                local_name,
                operator,
                value,
                never_matches,
                ..
            } => {
                if *never_matches {
                    compounds.last_mut().unwrap().never_matches = true;
                    continue;
                }
                let op = match operator {
                    AttrSelectorOperator::Equal => AttrOp::Equals(value.0.as_ref().to_string()),
                    _ => return Err(unsupported("attribute operators other than `=`")),
                };
                compounds.last_mut().unwrap().attrs.push(AttrMatch {
                    name: local_name.0.as_ref().to_string(),
                    op,
                });
            }
            Component::AttributeOther(_) => {
                return Err(unsupported("namespaced attribute selector"));
            }

            Component::Nth(data) => {
                if data.ty.is_of_type() {
                    return Err(unsupported(":nth-of-type / :first-of-type"));
                }
                if data.ty.is_from_end() {
                    if data.a == 0 && data.b == 1 && data.ty == NthType::LastChild {
                        compounds
                            .last_mut()
                            .unwrap()
                            .structural
                            .push(StructuralPseudo::LastChild);
                        continue;
                    }
                    return Err(unsupported(":nth-last-child / :last-of-type"));
                }
                if data.ty == NthType::Child {
                    if data.a == 0 && data.b == 1 {
                        compounds
                            .last_mut()
                            .unwrap()
                            .structural
                            .push(StructuralPseudo::FirstChild);
                    } else {
                        compounds
                            .last_mut()
                            .unwrap()
                            .structural
                            .push(StructuralPseudo::NthChild {
                                a: data.a,
                                b: data.b,
                            });
                    }
                } else if data.ty == NthType::OnlyChild {
                    return Err(unsupported(":only-child"));
                } else {
                    return Err(unsupported("Nth pseudo variant"));
                }
            }
            Component::NthOf(_) => return Err(unsupported(":nth-child(... of ...)")),

            Component::Negation(_) => return Err(unsupported(":not()")),
            Component::Where(_) => return Err(unsupported(":where()")),
            Component::Is(_) => return Err(unsupported(":is()")),
            Component::Has(_) => return Err(unsupported(":has()")),
            Component::Any(_, _) => return Err(unsupported(":any()")),

            Component::PseudoElement(_) => return Err(unsupported("pseudo-element ::*")),
            Component::NonTSPseudoClass(_) => {
                return Err(unsupported("non-tree-structural pseudo-class (e.g. :hover)"));
            }

            Component::Root | Component::Empty | Component::Scope | Component::Nesting => {
                return Err(unsupported(":root / :empty / :scope / nesting"));
            }
            Component::Slotted(_) | Component::Part(_) | Component::Host(_) => {
                return Err(unsupported("shadow-DOM selectors"));
            }
            Component::Namespace(_, _) => return Err(unsupported("namespaced selector")),
        }
    }

    Ok(MatchSelector {
        compounds,
        combinators,
        specificity,
    })
}

fn unsupported(feature: &str) -> LayoutError {
    LayoutError::UnsupportedCss {
        feature: feature.into(),
        location: None,
    }
}
