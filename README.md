# eguiwm

**eguiwm** is an experimental, speed-first Rust Wayland compositor.

The goal is a small hybrid window manager: tiling by default, optional floating windows, no animations, and a separate egui settings app that never sits in the compositor's rendering path.

> **Status: early prototype.** This first code drop is a nested Smithay compositor for safe development, with a simple tiling layout core. It is not ready to replace your desktop session. Native DRM/TTY startup, explicit floating-mode controls, workspaces, and the egui settings app are not implemented yet.

## Design goals

- Rust + [Smithay](https://github.com/Smithay/smithay) for Wayland compositor primitives.
- Small, deterministic layout calculations with unit tests.
- No animation engine or transition timers.
- Keep optional GUI tooling out of the compositor's hot path.
- Test in a nested session before adding a native DRM backend.

## Current prototype

The current backend uses Smithay's **winit nested backend**, so it runs inside an existing desktop session. This is deliberate: a broken compositor should not lock you out of your desktop while development is underway.

Toplevel windows are arranged in a master/stack layout when they appear. The nested backend is based on Smithay's Smallvil example; see `THIRD_PARTY_NOTICES.md`.

## Build prerequisites (Arch Linux)

Install Rust, the native libraries, and the libraries needed by the egui panel:

```sh
sudo pacman -S --needed rust cargo pkgconf wayland libxkbcommon libxkbcommon-x11
```

Depending on the Smithay version and graphics stack, additional system libraries may be needed by its transitive dependencies.

## Build and run

```sh
cargo build --release --bins
RUST_LOG=info cargo run --release
```

This opens a nested compositor window and starts the separate egui taskbar client when the panel binary is present. The panel includes four workspaces, launcher buttons, and basic load/memory information. It does **not** start a native Wayland session yet. Build all binaries first (`cargo build --release --bins`) so `eguiwm-panel` is available next to the compositor executable.

To launch a test client inside the nested compositor:

```sh
cargo run --release -- --command foot
```

Replace `foot` with an installed Wayland client. Do not use this as your login session yet.

## Performance target

The initial target is a **2-second startup**, measured from compositor process launch to a usable desktop. This is a target, not a benchmark result. We will report measured cold/warm startup, idle CPU, memory, and input latency before claiming performance.

## Roadmap

- [x] Smithay nested compositor skeleton
- [x] Pure layout module with unit tests
- [ ] Reflow windows when a client closes or the output resizes
- [ ] Keyboard shortcuts and explicit floating mode
- [x] Four basic workspaces with taskbar switching
- [ ] Multiple-output support
- [x] Separate egui taskbar with launcher buttons and basic system summary
- [ ] Damage-driven rendering and idle CPU benchmarks
- [ ] Native DRM/TTY backend
- [ ] Separate egui settings app
- [ ] CI: format, tests, and release build

## License

MIT. See [LICENSE](LICENSE).
