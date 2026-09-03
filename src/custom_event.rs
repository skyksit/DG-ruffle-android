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
    TogglePause,
    /// Flush all SharedObjects (.sol) to disk, then notify Java via the
    /// optional `onSharedObjectsFlushed()` callback.
    ///
    /// Not synchronous with the `flushSharedObjects()` JNI call that queues it:
    /// the write happens when the event loop next drains this event. A host
    /// that calls it from `onPause()` and is then killed may lose the save, so
    /// wait for `onSharedObjectsFlushed()` before assuming it is persisted.
    FlushSharedObjects,
}
