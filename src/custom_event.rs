//! Custom event type for Ruffle on Android

use ruffle_core::events::{KeyDescriptor, MouseButton};

/// User-defined events.
#[derive(Debug)]
pub enum RuffleEvent {
    /// Indicates that one or more tasks are ready to poll on our executor.
    TaskPoll,
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
