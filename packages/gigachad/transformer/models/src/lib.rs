#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "gen")]
pub mod gen;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum LayoutDirection {
    Row,
    #[default]
    Column,
}

impl std::fmt::Display for LayoutDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row => f.write_str("row"),
            Self::Column => f.write_str("column"),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum LayoutOverflow {
    Auto,
    Scroll,
    Show,
    #[default]
    Squash,
    Wrap,
}

impl std::fmt::Display for LayoutOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    #[default]
    Default,
}

impl std::fmt::Display for JustifyContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => f.write_str("start"),
            Self::Center => f.write_str("center"),
            Self::End => f.write_str("end"),
            Self::SpaceBetween => f.write_str("space-between"),
            Self::SpaceEvenly => f.write_str("space-evenly"),
            Self::Default => f.write_str("default"),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum AlignItems {
    Start,
    Center,
    End,
    #[default]
    Default,
}

impl std::fmt::Display for AlignItems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => f.write_str("start"),
            Self::Center => f.write_str("center"),
            Self::End => f.write_str("end"),
            Self::Default => f.write_str("default"),
        }
    }
}

#[cfg(feature = "calc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum LayoutPosition {
    Wrap {
        row: u32,
        col: u32,
    },
    #[default]
    Default,
}

#[cfg(feature = "calc")]
impl LayoutPosition {
    #[must_use]
    pub const fn row(&self) -> Option<u32> {
        match self {
            Self::Wrap { row, .. } => Some(*row),
            Self::Default => None,
        }
    }

    #[must_use]
    pub const fn column(&self) -> Option<u32> {
        match self {
            Self::Wrap { col, .. } => Some(*col),
            Self::Default => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SwapTarget {
    #[default]
    This,
    Children,
}

impl std::fmt::Display for SwapTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::This => f.write_str("self"),
            Self::Children => f.write_str("children"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Route {
    Get {
        route: String,
        trigger: Option<String>,
        swap: SwapTarget,
    },
    Post {
        route: String,
        trigger: Option<String>,
        swap: SwapTarget,
    },
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Cursor {
    #[default]
    Auto,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    EwResize,
    NsResize,
    NeswResize,
    ZoomIn,
    ZoomOut,
}

impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Pointer => f.write_str("pointer"),
            Self::Text => f.write_str("text"),
            Self::Crosshair => f.write_str("crosshair"),
            Self::Move => f.write_str("move"),
            Self::NotAllowed => f.write_str("not-allowed"),
            Self::NoDrop => f.write_str("no-drop"),
            Self::Grab => f.write_str("grab"),
            Self::Grabbing => f.write_str("grabbing"),
            Self::AllScroll => f.write_str("all-scroll"),
            Self::ColResize => f.write_str("col-resize"),
            Self::RowResize => f.write_str("row-resize"),
            Self::NResize => f.write_str("n-resize"),
            Self::EResize => f.write_str("e-resize"),
            Self::SResize => f.write_str("s-resize"),
            Self::WResize => f.write_str("w-resize"),
            Self::NeResize => f.write_str("ne-resize"),
            Self::NwResize => f.write_str("nw-resize"),
            Self::SeResize => f.write_str("se-resize"),
            Self::SwResize => f.write_str("sw-resize"),
            Self::EwResize => f.write_str("ew-resize"),
            Self::NsResize => f.write_str("ns-resize"),
            Self::NeswResize => f.write_str("nesw-resize"),
            Self::ZoomIn => f.write_str("zoom-in"),
            Self::ZoomOut => f.write_str("zoom-out"),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => f.write_str("static"),
            Self::Relative => f.write_str("relative"),
            Self::Absolute => f.write_str("absolute"),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Visible => f.write_str("visible"),
            Self::Hidden => f.write_str("hidden"),
        }
    }
}
