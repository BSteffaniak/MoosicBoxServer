#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

#[cfg(feature = "api")]
pub mod api;

pub mod library;
pub use moosicbox_menu_models as models;
