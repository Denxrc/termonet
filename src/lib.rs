//! Termonet: Fast, secure Material You system theme and palette generator for Linux.

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod i18n;
pub mod template;
pub mod tui;
pub mod wallpaper;

pub use error::{Result, TermonetError};
