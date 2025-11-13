//! Custom event type for Ruffle on Android

use ruffle_core::events::{KeyDescriptor, MouseButton};

use crate::PlayerRunnable;

/// User-defined events.
pub enum RuffleEvent {
    /// Indicates that a task is ready to be polled.
    TaskPoll(PlayerRunnable),
    VirtualKeyEvent {
        down: bool,
        key_descriptor: KeyDescriptor,
    },
    VirtualMouseEvent {
        down: bool,
        button: MouseButton,
    },
    RunContextMenuCallback(usize),
    ClearContextMenu,
    RequestContextMenu,
}
