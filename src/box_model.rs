#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size { width: 0.0, height: 0.0 };
    pub const fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sides<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy + Default> Sides<T> {
    pub const fn all(v: T) -> Self
    where
        T: Copy,
    {
        Sides { top: v, right: v, bottom: v, left: v }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxModel {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub padding: Sides<f32>,
    pub border: Sides<f32>,
    pub margin: Sides<f32>,
    /// Content size when overflow:scroll. None means no scroll.
    pub scroll: Option<Size>,
}
