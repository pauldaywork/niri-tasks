//! The task panel: each monitor's workspace tasks as notification-style cards
//! on the right edge, tucked away to a peek until hovered.

pub mod blur;
pub mod model;
pub mod style;
pub mod surface;

pub use surface::Panel;
