# Owerlayer 🦉 (v0.22.6)

*(c) 2026 omaquu*

Owerlayer is a high-performance, lightweight, transparent screen overlay application built in Rust, powered by `egui` and hardware-accelerated OpenGL. It allows you to draw, annotate, snip live windows, erase, transform, and overlay live content or shader effects directly over other running applications or games without disrupting your workflow.

---

## Key Features 🚀

- **Version 0.22.6 Suite**:
  - **Live & Desktop Snips**: Capture screen regions or desktop windows in real-time (up to 360 FPS) with marching ants outlines.
  - **Selected Source Outlines**: Source rect marching ants outlines display only when the snip object is actively selected.
  - **Interactive Source Resizing & Erasing**: Erase directly over source rectangles or snip images using pixel or connected flood-fill stroke eraser.
  - **Perspective & Inverse Mapping**: Real-time inverse quad perspective mapping ensures erasing and handle transforms align 100% with screen pixels.
  - **Multi-Layer Architecture & History**: Full action history tracking for layers, objects, text edits, and stroke modifications with non-blocking single-line history entries.
  - **Custom Photoshop-Style Toolbar**: Dedicated Main Color & Foreground Color pickers with integrated eyedroppers, transform reset, and source reset tools.
  - **Hardware-Accelerated Layer FX**: Custom OpenGL fragment shaders including Gaussian Blur, Pixelate, VHS Glitch, Chromatic Aberration, Glow, Grayscale, Invert, Sepia, and Drop Shadows.

---

## Keyboard Shortcuts ⌨️

| Shortcut | Action |
| --- | --- |
| `Ctrl + ~` *(Default)* | Toggle Edit Mode (Passthrough vs Focused Drawing) |
| `Ctrl + Z` | Undo |
| `Ctrl + Y` | Redo |
| `Ctrl + S` | Save current project state |
| `Delete` / `Backspace` | Erase active marquee selection or delete selected object |
| `Escape` | Cancel active drawing / marquee selection / text editing |
| `Enter` / `Right-Click` | Finalize custom Poly Blur or Poly Shape path |

---

## Building & GitHub Releases 🛠️

Automated Windows `.exe` builds are compiled via GitHub Actions on every commit to `main`.

To build locally:
```powershell
git clone https://github.com/omaquu/Owerlayer.git
cd Owerlayer
cargo build --release
```
The compiled executable will be located at `target/release/owerlayer.exe`.
