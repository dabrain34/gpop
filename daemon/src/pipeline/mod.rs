pub mod manager;
pub mod parser;

pub use manager::{PipelineInfo, PipelineManager};
pub use parser::Pipeline;

#[cfg(test)]
mod manager_tests;
