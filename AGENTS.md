# Owerlayer — Agent Guide

## Build

```powershell
cargo build --release
```

Release binary: `target/release/owerlayer.exe`  
Shipped copy: `releases/v0.18.0-alpha/owerlayer.exe`

## Key flows

1. **Edit mode** — hotkey toggles `edit_mode`; disables mouse passthrough and focuses overlay.
2. **Overlay setup** — `winapi_utils::setup_overlay_window()` runs on init and on passthrough changes.
3. **Live capture** — `capture_thread` + `wgc_capture` feed placed images marked `is_live`.

## Testing checklist

- Toggle edit mode with video playing behind overlay — desktop must keep animating.
- Drawing and layer tools work in edit mode.
- OBS captures via WGC window capture or game capture with transparency.
