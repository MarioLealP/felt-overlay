---
name: repo
type: repo
---

# felt-overlay — operating manual

This is a small, standalone desktop program: a normally-invisible,
click-through overlay that briefly appears on top of other windows when
Felt (the local AI assistant, `Benylin/felt-core` etc.) has something to
show, then recedes again — a "Shimeji-style" desktop companion. It is
**not** the main dashboard (`Benylin/felt-dashboard`) — that's a separate,
persistent embedded panel. Full design context:
`~/runbooks/assistant-overlay-companion.md` on Mario's machine (you don't
have access to it — this file is the self-contained summary of what
matters).

## Environment this runs in

- **Wayfire** (a wlroots-based Wayland compositor), not GNOME/KDE/Hyprland.
- The right way to build an always-on-top, click-through, positioned
  overlay here is the **wlr-layer-shell protocol**, not a normal toplevel
  window. `Benylin/wswitch` (a separate repo, an Alt+Tab daemon for this
  same desktop) is a **working reference example** of a layer-shell
  client in this exact environment — look at how it creates/positions its
  surface if you need a concrete pattern to follow, rather than guessing
  at layer-shell API usage from documentation alone.
- Transparency and click-through (input passes through to whatever's
  underneath when the overlay is idle/hidden) are hard requirements, not
  nice-to-haves — a companion that steals mouse focus or blocks clicks to
  the window behind it defeats the entire point.

## Scope: build ONLY Phase 1 right now

Per the design doc's own recommended sequencing: prototype the **overlay
mechanics** first, with the cheapest possible placeholder content, before
any real character rendering is attempted. Concretely, Phase 1 is:

1. A layer-shell surface, hidden by default, no persistent screen
   footprint while hidden.
2. A trigger mechanism to show it: a Unix socket (same pattern as
   `Benylin/felt-dashboard`'s `trigger_socket.rs` — a socket at
   `$XDG_RUNTIME_DIR/felt-overlay.sock` that accepts a short newline-
   terminated command) accepting at minimum `"show"` and `"hide"`.
3. A simple slide-in/slide-out animation on show/hide (from a screen
   edge is fine — exact easing/timing is your call, doesn't need to be
   pixel-perfect on the first pass).
4. **Placeholder content only** — a static image or a short looped
   video/GIF, whatever's simplest to get right with this toolkit. Do
   **not** attempt Live2D/VTube Studio compositing or a 3D character —
   that's explicitly a later phase, out of scope here.
5. Basic hide-after-N-seconds behavior (auto-hide a fixed delay after
   showing, in addition to an explicit `"hide"` command) so it doesn't
   sit on screen forever once shown.

**Explicitly out of scope for this task**: real character content,
wiring to Felt's actual brain/event stream (a later integration step,
once this mechanical shell is proven), any positioning/animation
polish beyond "it demonstrably works."

## What "done" looks like

- A README explaining how to build and run it, and how to test it
  manually (connect to the socket, send `show`/`hide`, confirm the
  overlay appears/disappears, confirm you can still click through it to
  whatever's underneath while it's showing).
- Unit tests for anything that's pure logic (socket command parsing, the
  show/hide/auto-hide state machine) — there is no way to unit-test the
  actual on-screen rendering/compositor behavior from here, so don't
  invent tests that pretend to. Say plainly in the README what still
  needs a human to actually look at the screen and confirm.
- A working build (document the exact build command).

## Workflow

- Work on `main` directly — this is a small greenfield repo, no staged
  dev/uat/prod promotion needed (unlike `nova-lemuria`, which is a real
  production website).
- Run whatever the project's own build/test/lint commands are before
  each push (add them to this repo as you establish them — there's
  nothing pre-existing to run yet on the very first task).
- Small, reviewable commits. If you get stuck or the layer-shell approach
  turns out not to work as expected, stop and say so clearly rather than
  forcing something that doesn't actually meet the click-through/
  always-on-top requirement.
