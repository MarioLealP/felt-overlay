# felt-overlay

A small, lightweight, click-through desktop companion overlay for [Felt](https://git.clockmost.com/Benylin/Felt).

Built for **Wayfire** (wlroots-based Wayland compositors) using the **wlr-layer-shell protocol**. It sits completely transparent and click-through on the `Overlay` layer, responding to IPC commands over a Unix domain socket, displaying a companion sprite with smooth slide-in and slide-out animations, and auto-hiding after a configurable delay.

## Requirements

- Linux with a Wayland compositor supporting `wlr-layer-shell` (e.g. Wayfire, labwc, sway).
- Rust toolchain (`cargo`, `rustc` 1.85+).
- System libraries: `libwayland-dev`, `libxkbcommon-dev`, `pkg-config`.

## Building

```bash
cargo build --release
```

The resulting binary is located at `target/release/felt-overlay`.

## Running

Launch the daemon:

```bash
cargo run --release
```

Or run the compiled binary directly:

```bash
./target/release/felt-overlay [OPTIONS]
```

### CLI Options

- `--socket <path>`: Custom Unix socket path (defaults to `$XDG_RUNTIME_DIR/felt-overlay.sock`).
- `--timeout <seconds>`: Duration in seconds to remain shown before auto-hiding (default: `5.0`).
- `--slide <milliseconds>`: Duration of the slide animation in milliseconds (default: `250`).
- `--image <path>`: Custom PNG sprite to use instead of the embedded companion placeholder.
- `--help, -h`: Show usage information.

## Manual Testing & Verification

The overlay daemon listens on a Unix domain socket at `$XDG_RUNTIME_DIR/felt-overlay.sock`. You can interact with it using `nc -U`, `socat`, or Python.

### 1. Show the overlay
```bash
echo show | nc -U "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/felt-overlay.sock"
```
The companion sprite slides into view smoothly at the bottom-right corner of the screen and stays for 5 seconds before sliding out automatically.

### 2. Hide immediately
```bash
echo hide | nc -U "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/felt-overlay.sock"
```
Triggers an immediate slide-out animation to hide the overlay.

### 3. Toggle visibility
```bash
echo toggle | nc -U "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/felt-overlay.sock"
```

### 4. Click-Through Verification
While the overlay is shown (or during the slide-in/slide-out animation), click on any window or desktop icon situated directly behind the sprite. All mouse clicks, scrolls, and drag events must pass directly through to the underlying application.

### 5. Running Automated Unit Tests
Pure logic components (socket command parser, animation/auto-hide state machine, sprite blitting/clipping, and socket IPC) are covered by unit tests:

```bash
cargo test
```

## What Still Requires Visual Confirmation on Screen

Automated tests cover state machine transitions, command parsing, socket IPC, and image blitting. The following aspects depend on compositor rendering and require visual confirmation on a physical or virtual desktop display:

1. **Overlay Visibility & Anchoring**: Visually confirm that the sprite appears anchored at the bottom-right corner on top of active windows when `show` is triggered.
2. **Animation Smoothness**: Visually verify that the slide-up entry and slide-down exit animations run smoothly on Wayfire.
3. **Click-Through Behavior**: Visually confirm that mouse clicks and hover events pass through the overlay to underlying windows without stealing focus.
4. **Zero Persistent Screen Footprint**: Confirm that when hidden, the overlay is completely transparent and leaves no visual footprint or exclusive margins on the desktop.

