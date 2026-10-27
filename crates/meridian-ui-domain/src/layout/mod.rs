//! Two-pass layout: measure (children report the size they want) then
//! arrange (parents hand out slots), as in WPF.

mod arrange;
mod border;
mod dock_panel;
mod fixed_advance;
mod frame;
mod grid;
mod measure;
mod pass;
mod stack_panel;
mod text_block;
mod text_measurer;
pub mod value;

pub use fixed_advance::FixedAdvanceText;
pub use pass::compute_layout;
pub use text_measurer::TextMeasurer;
