# Owerlayer 🦉

Owerlayer is a lightweight, transparent screen overlay application built in Rust, powered by `egui` and hardware-accelerated OpenGL. It allows you to draw, annotate, snip, and overlay live content or shader effects directly over other running applications or games without disrupting your workflow.

---

## Key Features 🚀

- **Edit Mode Toggle**: Smoothly switch between clicking through the overlay (passthrough mode) and focusing on the overlay to draw or edit (hotkey-activated).
- **Multi-Layer Annotations**: Create, reorder, group, lock, and hide layers to organize drawings.
- **Advanced Brushes**:
  - **Solid / Calligraphy**: High-precision lines with pressure-friendly rendering.
  - **Highlighter**: Semi-transparent highlighter with adjustable opacity.
  - **Spray**: Spray brush with adjustable density.
- **Dynamic Selection & Snipping**:
  - **Static Snips**: Capture screen regions and place them on the overlay.
  - **Live Snips**: Capture screen regions in real-time (up to 360 FPS via Windows Graphics Capture, falls back to GDI in Performance mode) to overlay active videos or animations.
- **Custom Shapes**: Draw Rectangles, Circles, Stars, Hearts, or manual point-to-point Polygons.
- **Hardware-Accelerated Layer FX**:
  - **Gaussian Blur, Pixelate, and VHS Glitch**: Apply custom fragment shaders to snips.
  - **Chromatic Aberration & Antialiasing**: Smooth out borders or apply chromatic offsets.
  - **Secondary Filters**: Grayscale, Sepia, Invert, Glow, and Shadow effects.
- **OBS / Capture Friendly**: Easy overlay transparency settings so you can stream or record with clear transparency via Game Capture or Windows Graphics Capture (WGC).

---

## Keyboard Shortcuts ⌨️

| Shortcut | Action |
| --- | --- |
| `Ctrl + ~` *(Default)* | Toggle Edit Mode (Passthrough vs Focused Drawing) |
| `Ctrl + Z` | Undo |
| `Ctrl + Y` | Redo |
| `Ctrl + S` | Save current project state |
| `Escape` | Cancel active drawing / marquee selection / text editing |
| `Enter` / `Right-Click` | Finalize custom Poly Blur or Poly Shape path |

---

## Building from Source 🛠️

Ensure you have [Rust and Cargo](https://rustup.rs/) installed on your system.

1. Clone the repository:
   ```powershell
   git clone https://github.com/omaquu/Owerlayer.git
   cd Owerlayer
   ```

2. Compile the application in Release mode:
   ```powershell
   cargo build --release
   ```

3. Run the compiled binary:
   ```powershell
   ./target/release/owerlayer.exe
   ```

---

## CI/CD Release Pipeline ⚙️

This repository has a built-in **GitHub Actions Release Pipeline** (`.github/workflows/release.yml`) that triggers automatically:
- On every **push to `main`**: Compiles the code and uploads the Windows executable as a build artifact.
- On **pushing tags** (e.g. `v0.17.0`): Compiles the binary, packages it, and automatically publishes a GitHub Release containing the standalone `owerlayer.exe` binary.

---

## Known Issues ⚠️

- **Widget Resizing**: Resizing widgets is currently buggy.
- **Deleting Widgets**: Deleting a widget crashes the application.
- **Lasso Marching Ants**: Marching ants outline for the Lasso tool is not fully connected.
- **Source Perspective Resizing**: When changing the perspective of the source, the object is not being resized to fit the new perspective.

