//!
//! The standard implementation of Focus
//!
//! Note, you only need to know about the `Focus` and `FocusEvent` messages to interact with this
//! subprogram. Typically there's only one Focus per window scene, so you only need to send messages
//! to the default target when working with a window. Focus itself isn't even dependent on flo_draw
//! so there's usually not even a need to start it manually when using with other windowing libraries.
//!
//! The functions in here can be used to create your own seperate Focus subprogram with independent
//! regions and event handling, should that ever be useful.
//!

mod subprogram;
mod button_state;
mod regions;
mod focus_state;
#[cfg(test)] mod test;

pub use subprogram::*;
