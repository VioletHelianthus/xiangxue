use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceSpan {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone)]
pub enum LayoutError {
    HtmlParse(String),
    CssParse(String),
    LayoutCompute(String),
    UnsupportedCss {
        feature: String,
        location: Option<SourceSpan>,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::HtmlParse(m) => write!(f, "html parse error: {m}"),
            LayoutError::CssParse(m) => write!(f, "css parse error: {m}"),
            LayoutError::LayoutCompute(m) => write!(f, "layout compute error: {m}"),
            LayoutError::UnsupportedCss { feature, location } => match location {
                Some(s) => write!(f, "unsupported CSS `{feature}` at {}:{}", s.line, s.column),
                None => write!(f, "unsupported CSS `{feature}`"),
            },
        }
    }
}

impl std::error::Error for LayoutError {}
