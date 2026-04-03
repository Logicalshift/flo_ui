use flo_draw::*;

use std::collections::*;

///
/// The current state of the pointer buttons
///
pub (super) struct ButtonState {
    /// Set of the buttons that are currently down
    is_down: HashSet<Button>,
}

impl ButtonState {
    ///
    /// Creates a new button state
    ///
    pub fn new() -> Self {
        ButtonState {
            is_down: HashSet::new()
        }
    }

    ///
    /// Updates the buttons that are down
    ///
    pub fn set_buttons<'a>(&mut self, pressed_buttons: impl IntoIterator<Item=&'a Button>) {
        self.is_down = pressed_buttons.into_iter().copied().collect();
    }

    ///
    /// Number of buttons that are down
    ///
    pub fn num_buttons_down(&self) -> usize {
        self.is_down.len()
    }
}
