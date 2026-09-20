//! Classify skills using metadata and load only the chosen instructions.
pub mod catalog;
pub mod editor;
mod form;
mod render;
mod scan;
pub mod storage;

pub type Result<T> = std::result::Result<T, String>;

#[cfg(test)]
#[path = "behavior_tests.rs"]
pub(crate) mod tests;
