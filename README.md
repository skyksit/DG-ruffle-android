This is a native Android application for [Ruffle](https://ruffle.rs).

It is in a very early stage.

# Prebuilt APKs

[<img src="https://fdroid.gitlab.io/artwork/badge/get-it-on.png"
     alt="Get it on F-Droid"
     height="75">](https://f-droid.org/packages/rs.ruffle/)

The latest release [(here)](https://github.com/ruffle-rs/ruffle-android/releases) should have a few `.apk` files uploaded as assets.

You can try this app by downloading and installing one of those.

- **For the vast majority of modern phones, tablets, single board computers, and small game consoles, you'll need the `arm64-v8a` version.**

- The `armeabi-v7a` version is for older, 32-bit ARM stuff.

- The `x86_64` version is for some rare Intel/Microsoft tablets and/or for Chromebooks, and/or for running on a PC on Android-x86 or in Waydroid or similar.

- The `x86` version is there mostly just for completeness.

- The `universal` version should work on all 4 of the above architectures, but it's _huge_.

# Building from source

Please see [CONTRIBUTING.md](CONTRIBUTING.md#building-from-source) for details about how to build this repository yourself.

---

# DGPlayer fork

This is the DGPlayer fork of `ruffle-rs/ruffle-android`. **The Android app in
this repository is not shipped.** The deliverable is the native library, built
per ABI, and consumed by a separate host app (`dgplayer-app/dsam3`):

```
dist/jniLibs/{arm64-v8a,armeabi-v7a,x86,x86_64}/libruffle_android.so
```

Build it with `./build-so.sh` (see `--help`; needs `cargo-ndk` and an NDK, and
autodetects the NDK from `local.properties`). `app/` is kept only as the
reference implementation of the JNI contract below — do not treat it as the
product.

## Host integration contract

JNI symbols are name-mangled, so the host class must be **exactly
`rs.ruffle.PlayerActivity`**, based on `GameActivity` (the crate uses
`android-activity` with the `game-activity` feature). A different package or
class name will not resolve.

Method resolution is lazy: an `external fun` with no matching symbol throws
`UnsatisfiedLinkError` at the *first call*, not at load. Conversely, the
methods Rust looks up are resolved eagerly in `nativeInit`, so a missing
**required** member kills the process at startup.

### Required members

`JavaInterface::init` (`src/java.rs`) resolves these on the class handed to
`nativeInit`. Each is `.expect(...)`, so a missing or renamed one aborts:

| Member | JNI signature |
| --- | --- |
| `getSurfaceWidth` | `()I` |
| `getSurfaceHeight` | `()I` |
| `showContextMenu` | `([Ljava/lang/String;)V` |
| `getSwfBytes` | `()[B` |
| `getSwfUri` | `()Ljava/lang/String;` |
| `getTraceOutput` | `()Ljava/lang/String;` |
| `getLocInWindow` | `()[I` |
| `getAndroidDataStorageDir` | `()Ljava/lang/String;` |

Plus the field `private val eventLoopHandle: Long = 0`, which `run()` owns via
`get_rust_field`/`take_rust_field`.

These are `private` and reachable only through JNI, so if the host ever enables
R8/minification it must `-keep` them.

### Optional callbacks

Both are looked up with `get_method_id(...).ok()` and the pending
`NoSuchMethodError` is cleared, so a host without them still starts:

| Callback | JNI signature | Notes |
| --- | --- | --- |
| `onContentReady` | `()V` | Fires **once per movie**, when the player starts playing. Called on the **event-loop thread, not the main thread** — post to your own looper before touching views. Do not call back into a native method that locks the player. Note "playing" precedes the first rendered frame, so expect a moment of black if you use it to hide a splash. |
| `onSharedObjectsFlushed` | `()V` | Completion signal for `flushSharedObjects()`. Fires even when no player existed, so it cannot distinguish "persisted" from "nothing to flush". |

### Native methods to declare

```kotlin
private external fun keydown(keyTag: String)
private external fun keyup(keyTag: String)
private external fun keydownByCode(keycode: Int)   // raw Android KeyEvent keycode
private external fun keyupByCode(keycode: Int)
private external fun mousedown(button: Int)        // 0=left, 1=right, 2=middle
private external fun mouseup(button: Int)
private external fun requestContextMenu()
private external fun runContextMenuCallback(index: Int)
private external fun clearContextMenu()
private external fun setMouseMode(mode: Int)          // 0=direct touch, 1=relative swipe
private external fun setTouchClickEnabled(enabled: Int)
private external fun setBackendMode(mode: Int)        // 0=Vulkan (default), 1=OpenGL
private external fun getActiveBackend(): Int          // 0=Vulkan, 1=OpenGL, -1=not created yet
private external fun getCursorPosition(): FloatArray? // [x, y] in View px, null=not touched yet
private external fun getCursorShape(): Int            // 0=Arrow 1=Hand 2=IBeam 3=Grab, -1=no frame yet
private external fun togglePause()
private external fun isPaused(): Int                  // 1=paused, 0=playing
private external fun flushSharedObjects()

companion object {
    @JvmStatic private external fun nativeInit(crashCallback: CrashCallback)
    @JvmStatic private external fun nativeCleanup()
}
```

### Lifecycle requirements

- **`nativeCleanup()` must be called from `onDestroy()`.** The panic hook
  installed by `nativeInit` captures a `GlobalRef` to the crash callback and
  `CRASH_CALLBACK_REF` holds a second one. Neither is released until
  `nativeCleanup()` runs, so skipping it leaks the Activity for the life of
  the process.
- **Renderer backend is a host-facing choice: `0` = Vulkan (default), `1` =
  OpenGL.** Note this is the opposite of what a `0 = OpenGL` convention would
  suggest — passing `0` gives Vulkan. The requested backend is honoured exactly
  (not as a `VULKAN|GL` mask), so "default is Vulkan" holds on every device.
- **`setBackendMode()` only takes effect before the renderer is built.** It is
  read once, when the player is constructed; later calls are stored (so a
  restart picks them up) and log a warning. Persist the choice yourself if you
  expose it as a setting.
- **Show `getActiveBackend()`, not the request.** If Vulkan was asked for but
  cannot initialise (old driver, blocklist, emulator), the renderer falls back
  to OpenGL rather than aborting, and `getActiveBackend()` then returns `1`
  while `setBackendMode` was given `0`. A toggle that echoes the request back
  would tell the user Vulkan is running when it is not. Returns `-1` until the
  renderer exists.
- **`flushSharedObjects()` is asynchronous.** It queues the flush; the write
  happens when the event loop next drains. Calling it from `onPause()` and
  then being killed can still lose the save — wait for
  `onSharedObjectsFlushed()` before assuming data is on disk.
- **`togglePause()` updates the pause flag synchronously**, so `isPaused()`
  immediately after it reports the new state. Applying it to the player still
  happens on the event-loop thread. The flag also survives surface recreation,
  so a paused movie stays paused across background/foreground.
- **Virtual mouse events need a cursor position.** `mousedown`/`mouseup` are
  dropped until the surface has been touched at least once, since there is no
  meaningful place to click before that.

### Drawing a cursor

Nothing is drawn for the virtual cursor. In Relative Swipe mode the finger and
the cursor are in different places, so a host that uses that mode should draw
its own pointer. `getCursorPosition()` and `getCursorShape()` exist for this;
both read a single atomic, take no lock, and are safe from any thread.

- **`getCursorPosition()` returns View-local pixels**, with the origin at the
  top-left of the SurfaceView whose size `getSurfaceWidth`/`getSurfaceHeight`
  report. Internally the cursor lives in surface-buffer pixels; the conversion
  is done natively because only the write site knows the buffer size. If the
  overlay's parent is not the SurfaceView's parent, add
  `surfaceView.getLocationInWindow(...)`.
- **`null` means the surface has never been touched** — exactly the state in
  which `mousedown`/`mouseup` are dropped. Hide the overlay (and any virtual
  click buttons) while it is null. A numeric sentinel is deliberately not used:
  in Direct Touch mode the position is unclamped, so negative and out-of-range
  coordinates are legitimate. Relative Swipe clamps to the surface bounds.
- **The position only changes on a touch event.** There is no change callback;
  poll from a `Choreographer` frame callback. Most frames return the same value.
- **`getCursorShape()` mirrors what the SWF asks for**, so a `Hand` result means
  the cursor is over a button or link — worth reflecting in the icon. Beware the
  naming: Ruffle's `Hand` is AS3 `MouseCursor.BUTTON` (pointing finger) and
  Ruffle's `Grab` is AS3 `MouseCursor.HAND` (grabbing hand).
- `flash.ui.Mouse.hide()` is **not** honoured — a SWF that draws its own cursor
  will show two. Say so if you hit it; supporting it needs a `UiBackend`.

### Behavioural differences from upstream

- Outbound navigation is disabled: `getURL`/`navigateToURL` are logged and
  ignored rather than opening a browser (`src/navigator.rs`).
- The renderer backend is selectable (Vulkan by default, OpenGL on request,
  with an automatic fallback to OpenGL if Vulkan is unavailable), where
  upstream hardcodes GL.
- Release builds log at `warn` and above; debug builds are verbose. Note this
  keys off the Cargo profile, and `cargoNdk` builds the release profile even
  for debug APKs.

---

# TODO

In no particular order:

- [ ] Ability to show the built-in virtual keyboard (softinput), for text input
- [ ] Controller/Gamepad input?
  - Mapped to key presses and/or virtual mouse pointer
- [ ] Own custom keyboard overlay, maybe even per-content configs
  - Not an overlay, and not per-content, but custom keyboard is there
- [ ] Error/panic handling
- [ ] Loading "animation" (spinner)
- [ ] Alternative audio backend (OpenSL ES) for Android < 8
- [ ] Proper storage backend?
- [ ] Resolve design glitches/styling/theming (immersive mode, window insets for holes/notches/corners)
- [ ] Publish to various app stores, maybe automatically?
- [ ] Bundle demo animations/games
- [ ] Add ability to load content from well known online collections? (well maybe not z0r... unless?)
- [ ] History, favorites, other flair...?

### DONE:

- [X] Clean up ~everything
- [X] Cross-platform build instructions?
  - I think gradle should take care of it now
- [X] UI backend (context menu)
  - Context menu works
- [X] Logging?
- [X] Navigator backend (fetch, open browser)
  - Opening links works at least
- [X] Touch/mouse input
- [X] Keyboard input: only with physical keyboard connected or through `scrcpy`
  - This was needed: https://github.com/rust-windowing/winit/pull/2226
- [X] Split into a separate repo
- [X] Add ability to Open SWF by entered/pasted URL (or even directly from clipboard)
  - No direct clipboard open, but easy to paste into the text field...
- [X] Unglitchify rendering: scale, center and letterbox the content properly
- [ ] Ask CPAL/Oboe to open a "media" type output stream instead of a "call" one
  - so the right volume slider controls it, and it uses the loud(er)speaker
  - -> solved by switching to a direct AAudio (ndk-audio) backend
- [X] Add building this to CI, at least to the release workflow
  - This repo has its own CI setup, which builds APKs
- [X] Simplify build process (hook cargo-apk into gradle, drop cargo-apk?)
  - ~cargo-apk is fine, but is only used to detect the SDK/NDK environment and run Cargo in it, and not to build an APK.~
  - actually solved by switching to `cargo-ndk` and the corresponding Gradle plugin
- [X] Somehow filter files to be picked to .swf
  - How well this works depends on the file picker, but it "should work most of the time"
- [X] Unglitchify audio volume (buttons unresponsive?)
  - (pending: https://github.com/rust-windowing/winit/pull/1919)
  - actually solved by switching to GameActivity instead
- [ ] Register Ruffle to open .swf files
  - How well this works depends on the application opening the file, but it "should work most of the time"
- [X] Figure out why videos are not playing (could be a seeking issue)
  - The video decoder features weren't enabled on `ruffle_core`...
- [X] Sign the APK
  - Using a very simple key for now, with just my name in it
- [X] Support for 32-bit ARM phones
  - Untested, but should work in theory
- [X] Support for x86(_64) tablets?
  - Sorted out
- [X] Consider not building the intermediate .apk just for the shared libraries
  - Figured out, no intermediate .apk any more, only native libs built
- [ ] Unbreak the regular build on CI
  - No longer relevant after the repo split
- [ ] Clean up commit history of the branch
  - No longer relevant after the repo split
