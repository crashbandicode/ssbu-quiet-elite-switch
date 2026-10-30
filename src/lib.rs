//! ssbu-quiet-elite-switch: on the Quickplay character select of Smash
//! Ultimate 13.0.5, switching from a fighter in Elite Smash to one that is not
//! shows a pop-up once per visit. This plugin marks it as already shown, so it
//! never appears; the screen still switches back from the Elite Smash colours.
//! UI only: no rating, network or match state changes.
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
pub mod share;
