pub mod model;
pub mod pdf;

#[cfg(target_arch = "wasm32")]
mod jobs;
#[cfg(target_arch = "wasm32")]
mod queue;
#[cfg(target_arch = "wasm32")]
mod telegram;
#[cfg(target_arch = "wasm32")]
mod webhook;

#[cfg(target_arch = "wasm32")]
pub use jobs::ConversionJob;
