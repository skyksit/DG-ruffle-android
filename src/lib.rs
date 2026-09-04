mod audio;
mod custom_event;
mod java;
mod keycodes;
mod navigator;
mod trace;

use custom_event::RuffleEvent;

use jni::{
    objects::{JObject, JString},
    sys::{self, jint, jobject},
    JNIEnv, JavaVM,
};
use keycodes::{
    android_key_event_to_ruffle_key_descriptor, key_tag_to_key_descriptor,
    keycode_to_key_descriptor,
};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, LazyLock, MutexGuard};
use std::time::Duration;
use std::{
    panic,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};
use wgpu::rwh::{AndroidDisplayHandle, HasWindowHandle, RawDisplayHandle};

use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use backtrace::Backtrace;
use jni::objects::{GlobalRef, JClass};

use audio::AAudioAudioBackend;
use url::Url;

use ruffle_common::duration::FloatDuration;
use ruffle_core::{
    backend::navigator::OwnedFuture,
    events::{LogicalKey, MouseButton, PlayerEvent},
    tag_utils::SwfMovie,
    Player, PlayerBuilder, ViewportDimensions,
};
use ruffle_frontend_utils::backends::storage::DiskStorageBackend;
use ruffle_frontend_utils::content::PlayingContent;
use ruffle_frontend_utils::{
    backends::navigator::{ExternalNavigatorBackend, FutureSpawner},
    content::ContentDescriptor,
};

use crate::navigator::AndroidNavigatorInterface;
use crate::trace::FileLogBackend;
use java::JavaInterface;
use ruffle_render_wgpu::{backend::WgpuRenderBackend, target::SwapChainTarget};

/// A unique identifier for a given `Player` instance.
/// Used to track which player any currently executing future is bound to.
#[derive(Copy, Clone, Eq, PartialEq)]
struct PlayerId(i64);

impl PlayerId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicI64, Ordering};

        static NEXT: AtomicI64 = AtomicI64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        assert!(id >= 0, "PlayerId overflowed!");
        Self(id)
    }
}

/// A `Player`-bound future that is currently running.
pub struct PlayerRunnable(async_task::Runnable<PlayerId>);

/// Represents a current Player and any associated state with that player,
/// which may be lost when this Player is closed (dropped)
struct ActivePlayer {
    id: PlayerId,
    player: Arc<Mutex<Player>>,
}

#[derive(Clone)]
pub struct EventSender {
    sender: Sender<RuffleEvent>,
    waker: AndroidAppWaker,
}

impl EventSender {
    pub fn send(&self, event: RuffleEvent) {
        if self.sender.send(event).is_ok() {
            self.waker.wake();
        }
    }
}

/// A bare-bones executor that schedules tasks on the winit event loop.
struct AndroidExecutor {
    event_loop: EventSender,
    player_id: PlayerId,
}

impl<E: std::error::Error + 'static> FutureSpawner<E> for AndroidExecutor {
    fn spawn(&self, future: OwnedFuture<(), E>) {
        // Discard any errors.
        let future = async {
            if let Err(e) = future.await {
                tracing::error!("Async error: {}", e);
            }
        };

        let event_loop = self.event_loop.clone();
        let scheduler = move |task| {
            let event = RuffleEvent::TaskPoll(PlayerRunnable(task));
            event_loop.send(event)
        };

        let (runnable, task) = async_task::Builder::new()
            .metadata(self.player_id)
            .spawn_local(|_| future, scheduler);

        // The future should run in the background.
        task.detach();
        // Immediately schedule the future to be polled for the first time.
        runnable.schedule();
    }
}
/// Whether Java has already been told that content is ready for the current
/// movie. Reset by `reset_player_state()` on every `run()`.
static CONTENT_READY_NOTIFIED: AtomicBool = AtomicBool::new(false);

/// Crash callback global reference, kept so `nativeCleanup` can release it.
static CRASH_CALLBACK_REF: Mutex<Option<GlobalRef>> = Mutex::new(None);

/// Last virtual cursor position, or `None` while the surface has not been
/// touched yet. Must not be a `(0.0, 0.0)` sentinel: that is a legitimate
/// position (top-left corner) and would make clicks there unreachable.
static LAST_MOUSE_POSITION: Mutex<Option<(f64, f64)>> = Mutex::new(None);

/// Mouse mode: 0 = Direct Touch (absolute), 1 = Relative Swipe (trackpad).
static MOUSE_MODE: AtomicU8 = AtomicU8::new(0);

/// Backend the host asked for: 0 = Vulkan (default), 1 = OpenGL.
/// Only read when the player is constructed; see `setBackendMode`. If Vulkan
/// cannot initialise the renderer falls back to OpenGL -- read `ACTIVE_BACKEND`
/// for what is actually running.
static BACKEND_MODE: AtomicU8 = AtomicU8::new(0);

/// Set once the renderer has been built, after which `BACKEND_MODE` no longer
/// has any effect on the running player.
static BACKEND_LOCKED_IN: AtomicBool = AtomicBool::new(false);

/// The backend actually in use: 0 = Vulkan, 1 = OpenGL, 255 = not yet decided.
/// May differ from `BACKEND_MODE` when Vulkan was requested but unavailable, so
/// a host that offers a backend toggle should display this, not the request.
static ACTIVE_BACKEND: AtomicU8 = AtomicU8::new(BACKEND_UNDECIDED);
const BACKEND_UNDECIDED: u8 = 255;

/// Per-pointer touch origin and cursor-at-touch-origin, for Relative Swipe
/// mode. Keyed by `Pointer::pointer_id()`, which is stable for the lifetime of
/// a finger -- unlike the pointer *index*, which shifts as fingers lift.
static TOUCH_STARTS: LazyLock<Mutex<HashMap<i32, (f64, f64)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static MOUSE_POS_AT_TOUCH_STARTS: LazyLock<Mutex<HashMap<i32, (f64, f64)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Touch click enabled: true = touch emits MouseDown/MouseUp, false = move only.
static TOUCH_CLICK_ENABLED: AtomicBool = AtomicBool::new(true);

/// Pause state: true = paused, false = playing.
static IS_PAUSED: AtomicBool = AtomicBool::new(false);

/// Reset the transient player state that must not leak between `run()` calls.
///
/// The `.so` is never unloaded, so these statics outlive an Activity. Without
/// this, a second launch in the same process would start with a stale cursor,
/// stale touch origins, and `CONTENT_READY_NOTIFIED` already set -- which
/// suppressed `onContentReady()` entirely on every launch after the first.
///
/// Host configuration is deliberately left alone; see the comment at the end.
fn reset_player_state() {
    CONTENT_READY_NOTIFIED.store(false, Ordering::SeqCst);
    IS_PAUSED.store(false, Ordering::SeqCst);
    *lock_poison_tolerant(&LAST_MOUSE_POSITION) = None;
    lock_poison_tolerant(&TOUCH_STARTS).clear();
    lock_poison_tolerant(&MOUSE_POS_AT_TOUCH_STARTS).clear();
    // A new renderer is about to be built, so the backend choice is live again.
    BACKEND_LOCKED_IN.store(false, Ordering::SeqCst);
    ACTIVE_BACKEND.store(BACKEND_UNDECIDED, Ordering::SeqCst);

    // MOUSE_MODE, TOUCH_CLICK_ENABLED and BACKEND_MODE are host *configuration*,
    // not player state, and the host may set them before run() starts (as the
    // reference Activity does for the backend). Resetting them here would
    // silently discard that.
}

/// These statics are only ever touched from the event-loop thread, so a
/// poisoned lock carries no cross-thread inconsistency worth propagating.
fn lock_poison_tolerant<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The last known cursor position, or `None` if the surface was never touched.
fn last_mouse_position() -> Option<(f64, f64)> {
    *lock_poison_tolerant(&LAST_MOUSE_POSITION)
}

fn set_last_mouse_position(pos: (f64, f64)) {
    *lock_poison_tolerant(&LAST_MOUSE_POSITION) = Some(pos);
}

/// Re-send the cursor position to the player.
///
/// Ruffle drops into touch mode and hides the cursor after non-pointer input,
/// so virtual-mouse and menu interactions have to re-prime the hover state.
/// No-op until the surface has actually been touched.
fn reprime_cursor(player: &Mutex<Player>) {
    if let Some((x, y)) = last_mouse_position() {
        lock_poison_tolerant(player).handle_event(PlayerEvent::MouseMove { x, y });
    }
}

/// Tell Java the content is ready, at most once per movie.
///
/// Must be called with no player lock held: this re-enters the JVM, and a host
/// implementation that calls back into a native method needing the player
/// would otherwise deadlock. It also runs on the event-loop thread, not the
/// main thread, so the host has to post to its own looper before touching UI.
fn notify_content_ready_once() {
    if CONTENT_READY_NOTIFIED.swap(true, Ordering::SeqCst) {
        return;
    }
    match get_jvm() {
        Ok((jvm, activity)) => match jvm.attach_current_thread() {
            Ok(mut env) => {
                JavaInterface::on_content_ready(&mut env, &activity);
                log::info!("Notified Java that content is ready");
            }
            Err(e) => {
                // Let a later tick retry rather than losing the signal.
                CONTENT_READY_NOTIFIED.store(false, Ordering::SeqCst);
                log::warn!("Could not attach thread to notify content ready: {e}");
            }
        },
        Err(e) => {
            CONTENT_READY_NOTIFIED.store(false, Ordering::SeqCst);
            log::warn!("JVM unavailable to notify content ready: {e}");
        }
    }
}

/// Post an event to the event loop via `PlayerActivity.eventLoopHandle`.
///
/// Returns whether the event was queued. Every JNI entry point must go through
/// this rather than unwrapping `get_rust_field`: `run()` takes the field back
/// during teardown, so a host calling in from `onPause`/`onDestroy` races that
/// window -- and a panic there would unwind out of `extern "C"` and abort the
/// whole process instead of failing one call.
///
/// # Safety
/// `this` must be the `PlayerActivity` whose `eventLoopHandle` was set by
/// `run()`, as for `JNIEnv::get_rust_field`.
unsafe fn post_event(env: &mut JNIEnv, this: &JObject, event: RuffleEvent) -> bool {
    // Resolve the borrow of `env` before touching it again on the error path.
    let outcome = match env.get_rust_field::<_, _, Sender<RuffleEvent>>(this, "eventLoopHandle") {
        Ok(event_loop) => Ok(event_loop.send(event).is_ok()),
        Err(e) => Err(e.to_string()),
    };

    match outcome {
        Ok(queued) => queued,
        Err(msg) => {
            // A failed field lookup can leave a pending exception; clear it so
            // the next JNI call on this thread is not poisoned by it.
            let _ = env.exception_clear();
            log::warn!("Event loop unavailable, dropping event: {msg}");
            false
        }
    }
}

#[tokio::main]
async fn run(app: AndroidApp) {
    let mut last_frame_time = Instant::now();
    let mut next_frame_time = Some(Instant::now());
    let mut quit = false;
    let (sender, receiver) = mpsc::channel::<RuffleEvent>();
    let mut native_window: Option<ndk::native_window::NativeWindow> = None;
    let mut playerbox: Option<ActivePlayer> = None;
    let sender = EventSender {
        sender,
        waker: app.create_waker(),
    };

    log::info!("Starting event loop...");
    let trace_output;
    let android_storage_dir;

    // The .so outlives the Activity, so clear everything a previous run left
    // behind before starting a new one.
    reset_player_state();
    log::info!("Player state reset: playing, cursor unknown");

    unsafe {
        let vm = JavaVM::from_raw(app.vm_as_ptr() as *mut sys::JavaVM).expect("JVM must exist");
        let activity = JObject::from_raw(app.activity_as_ptr() as jobject);
        let mut jni_env = vm.get_env().unwrap();
        trace_output = JavaInterface::get_trace_output(&mut jni_env, &activity);
        android_storage_dir = JavaInterface::get_android_data_storage_dir(&mut jni_env, &activity);
        let _ = jni_env.set_rust_field(activity, "eventLoopHandle", sender.clone());
    }

    while !quit {
        let mut needs_redraw = false;
        app.poll_events(
            Some(
                next_frame_time
                    .and_then(|next| next.checked_duration_since(last_frame_time))
                    .unwrap_or_else(|| Duration::from_millis(100)),
            ),
            |event| {
                match event {
                    PollEvent::Main(event) => match event {
                        MainEvent::Destroy => {
                            if let Some(player) = playerbox.as_ref() {
                                let mut player_lock = player.player.lock().unwrap();
                                player_lock.flush_shared_objects();
                            }
                            quit = true;
                        }
                        MainEvent::WindowResized { .. } => {
                            if let Some(player) = playerbox.as_ref() {
                                let mut player_lock = player.player.lock().unwrap();
                                let window = native_window
                                    .as_ref()
                                    .expect("native_window should be Some for a WindowResized");
                                log::info!(
                                    "WindowResized: {} x {}",
                                    window.width(),
                                    window.height()
                                );
                                let viewport_scale_factor = app
                                    .config()
                                    .density()
                                    .map(|dpi| dpi as f64 / 160.0)
                                    .unwrap_or(1.0);
                                let dimensions = ViewportDimensions {
                                    width: window.width() as u32,
                                    height: window.height() as u32,
                                    scale_factor: viewport_scale_factor,
                                };
                                player_lock.set_viewport_dimensions(dimensions);
                                needs_redraw = true;
                            }
                        }
                        MainEvent::Resume { .. } => {
                            if let Some(player) = playerbox.as_ref() {
                                if let Some(window) = native_window.as_ref() {
                                    // [NA] For some reason we can get negative sizes during a resume...
                                    if window.width() > 0 && window.height() > 0 {
                                        unsafe {
                                            let mut player = player
                                                .player
                                                .lock()
                                                .unwrap();

                                            let renderer = <dyn Any>::downcast_mut::<WgpuRenderBackend<SwapChainTarget>>(
                                                player.renderer_mut(),
                                            )
                                            .unwrap();

                                            renderer.recreate_surface_unsafe(
                                                wgpu::SurfaceTargetUnsafe::RawHandle {
                                                    raw_display_handle:
                                                        Some(RawDisplayHandle::Android(
                                                            AndroidDisplayHandle::new(),
                                                        )),
                                                    raw_window_handle: window
                                                        .window_handle()
                                                        .unwrap()
                                                        .into(),
                                                },
                                                (window.width() as u32, window.height() as u32),
                                            )
                                            .unwrap();
                                        }
                                    }
                                }
                            }
                        }
                        MainEvent::InitWindow { .. } => {
                            native_window = app.native_window();
                            let window = native_window
                                .as_ref()
                                .expect("native_window should be Some after InitWindow");
                            let viewport_scale_factor = app
                                .config()
                                .density()
                                .map(|dpi| dpi as f64 / 160.0)
                                .unwrap_or(1.0);
                            let dimensions = ViewportDimensions {
                                width: window.width() as u32,
                                height: window.height() as u32,
                                scale_factor: viewport_scale_factor,
                            };
                            log::info!(
                                "Init window: {} x {} (is existing: {})",
                                window.width(),
                                window.height(),
                                playerbox.is_some()
                            );

                            if let Some(activeplayer) = &playerbox {
                                let mut player_lock = activeplayer.player.lock().unwrap();
                                unsafe {
                                    let renderer = <dyn Any>::downcast_mut::<WgpuRenderBackend<SwapChainTarget>>(
                                        player_lock.renderer_mut(),
                                    )
                                    .unwrap();

                                    renderer.recreate_surface_unsafe(
                                        wgpu::SurfaceTargetUnsafe::RawHandle {
                                            raw_display_handle: Some(RawDisplayHandle::Android(
                                                AndroidDisplayHandle::new(),
                                            )),
                                            raw_window_handle: window
                                                .window_handle()
                                                .unwrap()
                                                .into(),
                                        },
                                        (window.width() as u32, window.height() as u32),
                                    )
                                    .unwrap();
                                }
                                // Respect an explicit pause across surface recreation
                                // (background -> foreground, rotation). Resuming
                                // unconditionally here would leave IS_PAUSED set while
                                // the movie runs, so the next togglePause() would
                                // "pause" by resuming.
                                let paused = IS_PAUSED.load(Ordering::SeqCst);
                                player_lock.set_is_playing(!paused);
                            } else {
                                // Backend mode: 0 = Vulkan (default), 1 = OpenGL.
                                //
                                // This is a user-facing choice in the host app, so the
                                // requested backend is honoured exactly rather than being
                                // passed as a VULKAN|GL mask -- with a mask wgpu is free to
                                // pick either adapter, which would make "default = Vulkan"
                                // untrue on some devices. Vulkan is tried alone, and only a
                                // genuine initialisation failure falls back to GL.
                                let backend_mode = BACKEND_MODE.load(Ordering::SeqCst);
                                BACKEND_LOCKED_IN.store(true, Ordering::SeqCst);
                                let requested = if backend_mode == 1 {
                                    wgpu::Backends::GL
                                } else {
                                    wgpu::Backends::VULKAN
                                };
                                log::info!("Requested renderer backend: {:?}", requested);

                                // SurfaceTargetUnsafe is not Clone, so build a fresh
                                // one per attempt.
                                let surface_target = || wgpu::SurfaceTargetUnsafe::RawHandle {
                                    raw_display_handle: Some(RawDisplayHandle::Android(
                                        AndroidDisplayHandle::new(),
                                    )),
                                    raw_window_handle: window
                                        .window_handle()
                                        .expect("native window must expose a raw handle")
                                        .into(),
                                };

                                let renderer = unsafe {
                                    // TODO: make this take an Arc<Window> instead?
                                    let attempt = WgpuRenderBackend::for_window_unsafe(
                                        surface_target(),
                                        (dimensions.width, dimensions.height),
                                        requested,
                                        wgpu::PowerPreference::HighPerformance,
                                        None,
                                    );
                                    match attempt {
                                        Ok(renderer) => {
                                            ACTIVE_BACKEND.store(backend_mode, Ordering::SeqCst);
                                            log::info!("Renderer backend in use: {:?}", requested);
                                            renderer
                                        }
                                        Err(e) if requested != wgpu::Backends::GL => {
                                            // Degrade instead of aborting the process: a
                                            // panic here would unwind out of extern "C".
                                            // The host asked for Vulkan and is not getting
                                            // it, so say so loudly.
                                            log::warn!(
                                                "Vulkan renderer unavailable ({e}); falling back to OpenGL"
                                            );
                                            let renderer = WgpuRenderBackend::for_window_unsafe(
                                                surface_target(),
                                                (dimensions.width, dimensions.height),
                                                wgpu::Backends::GL,
                                                wgpu::PowerPreference::HighPerformance,
                                                None,
                                            )
                                            .expect("OpenGL fallback renderer creation failed");
                                            ACTIVE_BACKEND.store(1, Ordering::SeqCst);
                                            log::info!(
                                                "Renderer backend in use: GL (fell back from Vulkan)"
                                            );
                                            renderer
                                        }
                                        Err(e) => {
                                            // edition 2018: panic! takes no implicit args.
                                            panic!("OpenGL renderer creation failed: {}", e)
                                        }
                                    }
                                };
                                let movie_url = Url::parse("file://movie.swf").unwrap();
                                let player_id = PlayerId::new();

                                let future_spawner = AndroidExecutor {
                                    event_loop: sender.clone(),
                                    player_id,
                                };

                                let navigator = ExternalNavigatorBackend::new(
                                    movie_url.clone(),
                                    None,
                                    None,
                                    future_spawner,
                                    None,
                                    true,
                                    Default::default(),
                                    ruffle_core::backend::navigator::SocketMode::Allow,
                                    Rc::new(PlayingContent::DirectFile(ContentDescriptor::new_remote(movie_url))),
                                    AndroidNavigatorInterface,
                                );

                                playerbox = Some(ActivePlayer {
                                    id: player_id,
                                    player: PlayerBuilder::new()
                                            .with_renderer(renderer)
                                            .with_audio(AAudioAudioBackend::new().unwrap())
                                            .with_storage(Box::new(DiskStorageBackend::new(android_storage_dir.clone())))
                                            .with_navigator(navigator)
                                            .with_log(FileLogBackend::new(trace_output.as_deref()))
                                            .with_video(
                                                ruffle_video_software::backend::SoftwareVideoBackend::new(),
                                            )
                                        .build(),
                                    }
                                );

                                let player = &playerbox.as_ref().unwrap().player;
                                let mut player_lock = player.lock().unwrap();
                                let (jvm, activity) = get_jvm().unwrap();
                                let mut env = jvm.attach_current_thread().unwrap();
                                let url = JavaInterface::get_swf_uri(&mut env, &activity);
                                let bytes = JavaInterface::get_swf_bytes(&mut env, &activity);

                                if let Some(bytes) = bytes {
                                    let movie = SwfMovie::from_data(&bytes, url, None, None).unwrap();
                                    player_lock.mutate_with_update_context(|context| {
                                        context.set_root_movie(movie);
                                    });
                                    // Notifying happens once, from the tick loop
                                    // below, after this lock is released -- see
                                    // notify_content_ready_once().
                                } else {
                                    player_lock.fetch_root_movie(url, Vec::new(), Box::new(|_| {}))
                                }
                                player_lock.set_is_playing(true); // Desktop player will auto-play.

                                player_lock.set_letterbox(ruffle_core::config::Letterbox::On);

                                player_lock.set_viewport_dimensions(dimensions);

                                last_frame_time = Instant::now();
                                next_frame_time = Some(Instant::now());

                                log::info!("MOVIE STARTED");
                            }
                        }
                        MainEvent::TerminateWindow { .. }  => {
                            let player = &playerbox.as_ref().unwrap().player;
                            let mut player_lock = player.lock().unwrap();
                            player_lock.set_is_playing(false);
                        }
                        MainEvent::InputAvailable => {
                            if let Ok(mut inputs) = app.input_events_iter() {
                                while inputs.next(|input| match input {
                                    InputEvent::MotionEvent(event) => {
                                        let window = native_window.as_ref().unwrap();
                                        let coords: (i32, i32) = get_loc_in_window();
                                        let view_size = get_view_size().unwrap();
                                        let action = event.action();
                                        // Mouse mode: 0 = Direct Touch, 1 = Relative Swipe
                                        let mouse_mode = MOUSE_MODE.load(Ordering::Relaxed);

                                        // Ruffle has a single cursor, so exactly one pointer
                                        // drives it. For down/up that is the pointer the
                                        // action refers to. For Move, `pointer_index()` is
                                        // always 0 and identifies nothing, so follow the
                                        // finger whose swipe is already being tracked.
                                        let (pointer_id, raw_x, raw_y) = {
                                            let acted_on = event.pointer_at_index(event.pointer_index());
                                            let fallback =
                                                (acted_on.pointer_id(), acted_on.x(), acted_on.y());
                                            if action == MotionAction::Move && mouse_mode != 0 {
                                                let tracked = lock_poison_tolerant(&TOUCH_STARTS);
                                                event
                                                    .pointers()
                                                    .find(|p| tracked.contains_key(&p.pointer_id()))
                                                    .map(|p| (p.pointer_id(), p.x(), p.y()))
                                                    .unwrap_or(fallback)
                                            } else {
                                                fallback
                                            }
                                        };

                                        let scaled_touch_x = (raw_x as f64 - coords.0 as f64)
                                            * window.width() as f64
                                            / view_size.0 as f64;
                                        let scaled_touch_y = (raw_y as f64 - coords.1 as f64)
                                            * window.height() as f64
                                            / view_size.1 as f64;

                                        let (mouse_x, mouse_y) = if mouse_mode == 0 {
                                            // Direct Touch mode: cursor jumps to the finger.
                                            (scaled_touch_x, scaled_touch_y)
                                        } else {
                                            // Relative Swipe mode: cursor accumulates the
                                            // delta from this finger's touch origin, so the
                                            // surface behaves like a trackpad.
                                            match action {
                                                MotionAction::Down
                                                | MotionAction::PointerDown
                                                | MotionAction::ButtonPress => {
                                                    // Anchor this finger: remember where it
                                                    // went down and where the cursor was.
                                                    let anchor = last_mouse_position()
                                                        .unwrap_or((scaled_touch_x, scaled_touch_y));
                                                    lock_poison_tolerant(&TOUCH_STARTS)
                                                        .insert(pointer_id, (scaled_touch_x, scaled_touch_y));
                                                    lock_poison_tolerant(&MOUSE_POS_AT_TOUCH_STARTS)
                                                        .insert(pointer_id, anchor);
                                                    // Touching down must not move the cursor.
                                                    anchor
                                                }
                                                MotionAction::Move => {
                                                    let start = lock_poison_tolerant(&TOUCH_STARTS)
                                                        .get(&pointer_id)
                                                        .copied();
                                                    match start {
                                                        Some((start_x, start_y)) => {
                                                            let anchor =
                                                                lock_poison_tolerant(&MOUSE_POS_AT_TOUCH_STARTS)
                                                                    .get(&pointer_id)
                                                                    .copied()
                                                                    .or_else(last_mouse_position)
                                                                    .unwrap_or((start_x, start_y));
                                                            let x = (anchor.0 + scaled_touch_x - start_x)
                                                                .clamp(0.0, window.width() as f64);
                                                            let y = (anchor.1 + scaled_touch_y - start_y)
                                                                .clamp(0.0, window.height() as f64);
                                                            (x, y)
                                                        }
                                                        // Untracked finger: leave the cursor be.
                                                        None => last_mouse_position()
                                                            .unwrap_or((scaled_touch_x, scaled_touch_y)),
                                                    }
                                                }
                                                MotionAction::Up
                                                | MotionAction::PointerUp
                                                | MotionAction::ButtonRelease => {
                                                    lock_poison_tolerant(&TOUCH_STARTS)
                                                        .remove(&pointer_id);
                                                    lock_poison_tolerant(&MOUSE_POS_AT_TOUCH_STARTS)
                                                        .remove(&pointer_id);
                                                    last_mouse_position()
                                                        .unwrap_or((scaled_touch_x, scaled_touch_y))
                                                }
                                                _ => last_mouse_position()
                                                    .unwrap_or((scaled_touch_x, scaled_touch_y)),
                                            }
                                        };

                                        set_last_mouse_position((mouse_x, mouse_y));

                                        // Check if touch click is enabled
                                        let touch_click_enabled = TOUCH_CLICK_ENABLED.load(Ordering::Relaxed);

                                        let ruffle_event = match event.action() {
                                            MotionAction::Down | MotionAction::PointerDown | MotionAction::ButtonPress => {
                                                if touch_click_enabled {
                                                    PlayerEvent::MouseDown {
                                                        x: mouse_x,
                                                        y: mouse_y,
                                                        button: MouseButton::Left, // TODO
                                                        index: None, // TODO
                                                    }
                                                } else {
                                                    PlayerEvent::MouseMove { x: mouse_x, y: mouse_y }
                                                }
                                            }
                                            MotionAction::Up | MotionAction::PointerUp | MotionAction::ButtonRelease => {
                                                if touch_click_enabled {
                                                    PlayerEvent::MouseUp {
                                                        x: mouse_x,
                                                        y: mouse_y,
                                                        button: MouseButton::Left, // TODO
                                                    }
                                                } else {
                                                    PlayerEvent::MouseMove { x: mouse_x, y: mouse_y }
                                                }
                                            }
                                            MotionAction::Move => PlayerEvent::MouseMove { x: mouse_x, y: mouse_y },
                                            _ => return InputStatus::Unhandled,
                                        };

                                        if let Some(player) = playerbox.as_ref() {
                                            player
                                                .player
                                                .lock()
                                                .unwrap()
                                                .handle_event(ruffle_event);
                                        }

                                        InputStatus::Handled
                                    }
                                    InputEvent::KeyEvent(event) => {
                                        if let Some(player) = playerbox.as_ref() {
                                            let Some(key_descriptor) =
                                                android_key_event_to_ruffle_key_descriptor(event)
                                            else {
                                                return InputStatus::Unhandled;
                                            };
                                            let down;
                                            let ruffle_event = match event.action() {
                                                KeyAction::Down => {
                                                    down = true;
                                                    PlayerEvent::KeyDown {
                                                        key: key_descriptor,
                                                    }
                                                }
                                                KeyAction::Up => {
                                                    down = false;
                                                    PlayerEvent::KeyUp { key: key_descriptor }
                                                }
                                                _ => return InputStatus::Unhandled,
                                            };
                                            player
                                                .player
                                                .lock()
                                                .unwrap()
                                                .handle_event(ruffle_event);

                                            // TODO: Use `KeyEvent.unicode_char` when it's available:
                                            // https://github.com/rust-mobile/android-activity/issues/183
                                            if down {
                                                if let LogicalKey::Character(c) = key_descriptor.logical_key {
                                                    let event = PlayerEvent::TextInput { codepoint: c };
                                                    player.player.lock().unwrap().handle_event(event);
                                                }
                                            };

                                            needs_redraw = true;
                                        }

                                        InputStatus::Handled
                                    }
                                    _ => InputStatus::Unhandled,
                                }) {}
                            }
                        }
                        _ => {} // Something else happened but it's probably not important for now.
                    },
                    PollEvent::Wake => {} // A task tried to wake us, we'll recv it below
                    PollEvent::Timeout => {} // No events happened, we'll tick as normal below
                    _ => {}               // Unknown future event
                }
            },
        );

        match receiver.try_recv() {
            Err(_) => {}
            Ok(RuffleEvent::TaskPoll(task)) => {
                // Only run the task if it matches our current player;
                // otherwise it is stale, and should be cancelled (which
                // happens implicitly on drop).
                if let Some(player) = playerbox.as_ref() {
                    if *task.0.metadata() == player.id {
                        task.0.run();
                    }
                }
            }
            Ok(RuffleEvent::VirtualKeyEvent {
                down,
                key_descriptor,
            }) => {
                if let Some(player) = playerbox.as_ref() {
                    // Keep the cursor primed around the key event.
                    reprime_cursor(&player.player);

                    let event = if down {
                        PlayerEvent::KeyDown {
                            key: key_descriptor,
                        }
                    } else {
                        PlayerEvent::KeyUp {
                            key: key_descriptor,
                        }
                    };
                    player.player.lock().unwrap().handle_event(event);

                    if down {
                        // TODO: Add shift/capslock and pass in uppercase characters accordingly
                        if let LogicalKey::Character(c) = key_descriptor.logical_key {
                            let event = PlayerEvent::TextInput { codepoint: c };
                            player.player.lock().unwrap().handle_event(event);
                        }
                    }

                    // Re-prime after the key event too -- matters most after key up.
                    reprime_cursor(&player.player);

                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::VirtualMouseEvent { down, button }) => {
                if let Some(player) = playerbox.as_ref() {
                    // A virtual click needs somewhere to click. Before the surface
                    // has ever been touched there is no cursor position, so there
                    // is nothing meaningful to dispatch.
                    if let Some((x, y)) = last_mouse_position() {
                        reprime_cursor(&player.player);

                        let event = if down {
                            PlayerEvent::MouseDown {
                                x,
                                y,
                                button,
                                index: None,
                            }
                        } else {
                            PlayerEvent::MouseUp { x, y, button }
                        };
                        player.player.lock().unwrap().handle_event(event);

                        // Re-prime after the click too -- matters most after mouse up.
                        reprime_cursor(&player.player);
                    }

                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::RunContextMenuCallback(index)) => {
                if let Some(player) = playerbox.as_ref() {
                    player
                        .player
                        .lock()
                        .unwrap()
                        .run_context_menu_callback(index);

                    // Keep the cursor primed after the menu selection.
                    reprime_cursor(&player.player);

                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::ClearContextMenu) => {
                if let Some(player) = playerbox.as_ref() {
                    player.player.lock().unwrap().clear_custom_menu_items();

                    // Keep the cursor primed after closing the menu.
                    reprime_cursor(&player.player);

                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::RequestContextMenu) => {
                if let Some(player) = playerbox.as_ref() {
                    log::warn!("preparing context menu!");
                    let items = player.player.lock().unwrap().prepare_context_menu();
                    let (jvm, activity) = get_jvm().unwrap();
                    let mut env = jvm.attach_current_thread().unwrap();
                    JavaInterface::show_context_menu(&mut env, &activity, &items);

                    // Keep the cursor primed after requesting the menu.
                    reprime_cursor(&player.player);

                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::TogglePause) => {
                if let Some(player) = playerbox.as_ref() {
                    // The flag was already flipped by the JNI entry point; this
                    // only applies it. Reading it here (rather than toggling
                    // again) also keeps repeated taps idempotent if several
                    // events coalesce.
                    let paused = IS_PAUSED.load(Ordering::SeqCst);
                    lock_poison_tolerant(&player.player).set_is_playing(!paused);

                    log::info!("Game {}", if paused { "paused" } else { "resumed" });
                    needs_redraw = true;
                }
            }
            Ok(RuffleEvent::FlushSharedObjects) => {
                if let Some(player) = playerbox.as_ref() {
                    player.player.lock().unwrap().flush_shared_objects();
                    log::info!("Shared objects flushed on request");
                }
                // Always notify Java so a waiting caller is released even
                // when no player exists yet.
                if let Ok((jvm, activity)) = get_jvm() {
                    if let Ok(mut env) = jvm.attach_current_thread() {
                        JavaInterface::on_shared_objects_flushed(&mut env, &activity);
                    }
                }
            }
        }

        let new_time = Instant::now();
        let dt = new_time.duration_since(last_frame_time).as_micros();
        if dt > 0 {
            last_frame_time = new_time;
            if let Some(player) = playerbox.as_ref() {
                let mut content_ready = false;
                if let Ok(mut player) = player.player.lock() {
                    player.tick(FloatDuration::from_millis(dt as f64 / 1000.0));
                    next_frame_time = Some(new_time + player.time_til_next_frame());
                    needs_redraw = player.needs_render();
                    let audio =
                        <dyn Any>::downcast_mut::<AAudioAudioBackend>(player.audio_mut()).unwrap();
                    audio.recreate_stream_if_needed();

                    content_ready = player.is_playing();
                }
                // Deliberately outside the lock above.
                if content_ready {
                    notify_content_ready_once();
                }
            } else {
                next_frame_time = None;
            }
        }

        if needs_redraw {
            if let Some(player) = playerbox.as_ref() {
                if let Ok(mut player) = player.player.lock() {
                    player.render();
                }
            }
        }
    }

    unsafe {
        let vm = JavaVM::from_raw(app.vm_as_ptr() as *mut sys::JavaVM).expect("JVM must exist");
        let activity = JObject::from_raw(app.activity_as_ptr() as jobject);
        // Ensure that we take the EventSender back, or we'll leak it
        let _: Result<EventSender, _> = vm
            .get_env()
            .unwrap()
            .take_rust_field(activity, "eventLoopHandle");
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_keydown(
    mut env: JNIEnv,
    this: JObject,
    key_tag: JString,
) {
    let tag: String = env
        .get_string(&key_tag)
        .expect("Couldn't get java string!")
        .into();

    if let Some(desc) = key_tag_to_key_descriptor(&tag) {
        post_event(
            &mut env,
            &this,
            RuffleEvent::VirtualKeyEvent {
                down: true,
                key_descriptor: desc,
            },
        );
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_keyup(
    mut env: JNIEnv,
    this: JObject,
    key_tag: JString,
) {
    let tag: String = env
        .get_string(&key_tag)
        .expect("Couldn't get java string!")
        .into();

    if let Some(desc) = key_tag_to_key_descriptor(&tag) {
        post_event(
            &mut env,
            &this,
            RuffleEvent::VirtualKeyEvent {
                down: false,
                key_descriptor: desc,
            },
        );
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_keydownByCode(
    mut env: JNIEnv,
    this: JObject,
    keycode: jint,
) {
    if let Some(desc) = keycode_to_key_descriptor(keycode) {
        post_event(
            &mut env,
            &this,
            RuffleEvent::VirtualKeyEvent {
                down: true,
                key_descriptor: desc,
            },
        );
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_keyupByCode(
    mut env: JNIEnv,
    this: JObject,
    keycode: jint,
) {
    if let Some(desc) = keycode_to_key_descriptor(keycode) {
        post_event(
            &mut env,
            &this,
            RuffleEvent::VirtualKeyEvent {
                down: false,
                key_descriptor: desc,
            },
        );
    }
}

/// Convert button code to MouseButton
/// 0 = Left, 1 = Right, 2 = Middle
fn button_code_to_mouse_button(button_code: jint) -> MouseButton {
    match button_code {
        0 => MouseButton::Left,
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => MouseButton::Left, // Default to left button
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_mousedown(
    mut env: JNIEnv,
    this: JObject,
    button_code: jint,
) {
    post_event(
        &mut env,
        &this,
        RuffleEvent::VirtualMouseEvent {
            down: true,
            button: button_code_to_mouse_button(button_code),
        },
    );
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_mouseup(
    mut env: JNIEnv,
    this: JObject,
    button_code: jint,
) {
    post_event(
        &mut env,
        &this,
        RuffleEvent::VirtualMouseEvent {
            down: false,
            button: button_code_to_mouse_button(button_code),
        },
    );
}

pub fn get_jvm<'a>() -> Result<(jni::JavaVM, JObject<'a>), Box<dyn std::error::Error>> {
    // Create a VM for executing Java calls
    let context = ndk_context::android_context();
    let activity = unsafe { JObject::from_raw(context.context().cast()) };
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) }?;

    Ok((vm, activity))
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_requestContextMenu(
    mut env: JNIEnv,
    this: JObject,
) {
    post_event(&mut env, &this, RuffleEvent::RequestContextMenu);
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_runContextMenuCallback(
    mut env: JNIEnv,
    this: JObject,
    index: jint,
) {
    post_event(
        &mut env,
        &this,
        RuffleEvent::RunContextMenuCallback(index as usize),
    );
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_clearContextMenu(
    mut env: JNIEnv,
    this: JObject,
) {
    post_event(&mut env, &this, RuffleEvent::ClearContextMenu);
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_setMouseMode(
    _env: JNIEnv,
    _this: JObject,
    mode: jint,
) {
    // mode: 0 = Direct Touch, 1 = Relative Swipe
    match mode {
        0 | 1 => {
            MOUSE_MODE.store(mode as u8, Ordering::Relaxed);
            log::info!(
                "Mouse mode: {}",
                if mode == 0 {
                    "Direct Touch"
                } else {
                    "Relative Swipe"
                }
            );
        }
        other => log::warn!("Ignoring unknown mouse mode {other}; expected 0 or 1"),
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_setTouchClickEnabled(
    _env: JNIEnv,
    _this: JObject,
    enabled: jint,
) {
    // enabled: 0 = disabled (only mouse move), 1 = enabled (mouse click on touch)
    let is_enabled = enabled != 0;
    TOUCH_CLICK_ENABLED.store(is_enabled, Ordering::Relaxed);
    log::info!(
        "Touch click events: {}",
        if is_enabled { "enabled" } else { "disabled" }
    );
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_setBackendMode(
    _env: JNIEnv,
    _this: JObject,
    mode: jint,
) {
    // mode: 0 = Vulkan (default), 1 = OpenGL
    match mode {
        0 | 1 => {
            BACKEND_MODE.store(mode as u8, Ordering::SeqCst);
            let name = if mode == 1 { "OpenGL" } else { "Vulkan" };
            if BACKEND_LOCKED_IN.load(Ordering::SeqCst) {
                // The renderer reads this once, at construction. Storing it is
                // still worthwhile: a restart in this process picks it up.
                log::warn!("Backend mode set to {name}, but the renderer already exists -- takes effect on next start");
            } else {
                log::info!("Backend mode: {name}");
            }
        }
        other => log::warn!("Ignoring unknown backend mode {other}; expected 0 or 1"),
    }
}

/// Which backend the renderer actually ended up using.
///
/// Returns 0 = Vulkan, 1 = OpenGL, -1 = renderer not created yet. This can
/// differ from the value passed to `setBackendMode` when Vulkan was requested
/// but could not initialise, so a host showing a backend toggle should read
/// this rather than echoing the request back to the user.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_getActiveBackend(
    _env: JNIEnv,
    _this: JObject,
) -> jint {
    match ACTIVE_BACKEND.load(Ordering::SeqCst) {
        BACKEND_UNDECIDED => -1,
        other => other as jint,
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_togglePause(mut env: JNIEnv, this: JObject) {
    // Flip the flag here, synchronously, so a host that calls isPaused()
    // straight after this gets the new state. Applying it to the player has to
    // happen on the event-loop thread, and that lags by at least one loop
    // iteration -- the loop drains one event per iteration -- so deciding the
    // new state over there would make isPaused() report the pre-toggle value.
    let paused = !IS_PAUSED.fetch_xor(true, Ordering::SeqCst);
    log::info!(
        "Pause requested: {}",
        if paused { "paused" } else { "playing" }
    );

    // If the loop is already gone the flag still reflects intent, and
    // InitWindow honours it if a player comes back.
    post_event(&mut env, &this, RuffleEvent::TogglePause);
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_flushSharedObjects(
    mut env: JNIEnv,
    this: JObject,
) {
    post_event(&mut env, &this, RuffleEvent::FlushSharedObjects);
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_isPaused(
    _env: JNIEnv,
    _this: JObject,
) -> jint {
    if IS_PAUSED.load(Ordering::SeqCst) {
        1
    } else {
        0
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_nativeInit(
    mut env: JNIEnv,
    class: JClass,
    crash_callback: JObject,
) {
    let crash_callback_ref = env.new_global_ref(crash_callback).unwrap();

    // Keep a reference so nativeCleanup() can release it.
    if let Ok(mut stored_ref) = CRASH_CALLBACK_REF.lock() {
        *stored_ref = Some(crash_callback_ref.clone());
    }

    let crash_callback = crash_callback_ref;
    let jvm = env.get_java_vm().unwrap();

    // Debug builds log verbosely; release builds keep warnings and errors.
    #[cfg(debug_assertions)]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("ruffle")
            .with_filter(
                android_logger::FilterBuilder::new()
                    .parse("warn,ruffle=info,wgpu_hal=info,wgpu_hal::gles=off,wgpu_core=info,symphonia_bundle_mp3=error,ruffle_core::tag_utils=error")
                    .build(),
            ),
    );

    // Release keeps warnings and errors. Turning logging fully off also
    // silenced the panic hook's `log::error!(target: "panic", ...)`, and with
    // `[profile.release] strip = "symbols"` that left field crashes with
    // neither a message nor a symbolized backtrace.
    //
    // Note this is keyed on the Cargo profile, not the Android build type:
    // cargoNdk builds the release profile even for debug APKs, so a debug APK
    // gets this branch too.
    #[cfg(not(debug_assertions))]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Warn)
            .with_tag("ruffle"),
    );

    panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::new();
        let thread = thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");
        let message = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        let full = match info.location() {
            Some(location) => format!(
                "thread '{}' panicked at '{}': {}:{}\n{:?}",
                thread,
                message,
                location.file(),
                location.line(),
                backtrace
            ),
            None => format!(
                "thread '{}' panicked at '{}'\n{:?}",
                thread, message, backtrace
            ),
        };
        log::error!(target: "panic","{}", full);

        let mut env = jvm.attach_current_thread().unwrap();
        if env.exception_check().unwrap() {
            // There's a pending exception, java will discover this on their own
        } else {
            let java_message = env.new_string(full).unwrap();
            let crash_callback = env.new_global_ref(&crash_callback).unwrap();
            env.call_method(
                crash_callback,
                "onCrash",
                "(Ljava/lang/String;)V",
                &[(&java_message).into()],
            )
            .unwrap();
        }
    }));

    JavaInterface::init(&mut env, &class)
}

/// Release the JNI global references held for crash reporting.
///
/// **The host must call this from `Activity.onDestroy()`.** The panic hook
/// installed by `nativeInit` captures a `GlobalRef` to the crash callback, and
/// `CRASH_CALLBACK_REF` holds a second one; neither is released until this
/// runs, so skipping it keeps the Activity alive for the life of the process.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn Java_rs_ruffle_PlayerActivity_nativeCleanup(_env: JNIEnv, _class: JClass) {
    log::info!("nativeCleanup called - releasing Global References");

    // Dropping the hook releases the crash_callback its closure captured.
    let _ = panic::take_hook();
    log::info!("Panic hook reset to default");

    // Then release the copy stored for exactly this purpose.
    if let Ok(mut stored_ref) = CRASH_CALLBACK_REF.lock() {
        if let Some(global_ref) = stored_ref.take() {
            drop(global_ref);
            log::info!("Crash callback Global Reference released");
        }
    }

    log::info!("nativeCleanup completed successfully");
}

fn get_loc_in_window() -> (i32, i32) {
    let (jvm, activity) = get_jvm().unwrap();
    let mut env = jvm.attach_current_thread().unwrap();

    // no worky :(
    //ndk_glue::native_activity().show_soft_input(true);

    JavaInterface::get_loc_in_window(&mut env, &activity)
}

fn get_view_size() -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let (jvm, activity) = get_jvm()?;
    let mut env = jvm.attach_current_thread()?;

    let width = JavaInterface::get_surface_width(&mut env, &activity);
    let height = JavaInterface::get_surface_height(&mut env, &activity);

    Ok((width, height))
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    log::info!("Starting android_main...");
    run(app);
}
