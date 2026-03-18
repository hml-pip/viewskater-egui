# viewskater-egui

An egui reimplementation of [viewskater](https://github.com/ggand0/viewskater).

## Features

- Dual pane mode with synced navigation
- Drag-and-drop files/folders onto panes
- Keyboard and slider navigation with natural sort ordering
- Skate mode for continuous scrolling through images
- Scroll-to-zoom centered on cursor, click-drag to pan, double-click to reset
- Sliding window cache with background preloading and LRU decode cache
- Supports jpg, png, bmp, webp, gif, tiff, qoi, tga

## Usage

```bash
cargo run --profile opt-dev -- /path/to/images/
cargo run --profile opt-dev -- /path/to/image.jpg
cargo run --profile opt-dev  # launch empty, drag-and-drop to open
```

Set `RUST_LOG=viewskater_egui=debug` for debug logging.

## Controls

| Input | Action |
|---|---|
| A / D or Left / Right arrow | Previous / next image |
| Hold A / D or arrows | Skate mode (continuous scroll) |
| Home / End | First / last image |
| Ctrl+1 / Ctrl+2 | Single / dual pane |
| Scroll wheel | Zoom (centered on cursor) |
| Click + drag | Pan |
| Double-click | Reset zoom and pan |
| Slider | Jump to position |
| F11 / Escape | Toggle / exit fullscreen |

## Linux Desktop Integration

On GNOME 46+ (Ubuntu 24.04+), the taskbar icon requires a `.desktop` file and icon installed to standard XDG locations. Without this, GNOME shows a generic gear icon.

```bash
# Install icon
mkdir -p ~/.local/share/icons/hicolor/256x256/apps
cp assets/icon_256.png ~/.local/share/icons/hicolor/256x256/apps/viewskater-egui.png
gtk-update-icon-cache -f ~/.local/share/icons/hicolor/

# Install desktop entry (edit Exec= path to match your setup)
cp assets/viewskater-egui.desktop ~/.local/share/applications/
```

If using the AppImage, update the `Exec=` line to point to the AppImage path:

```bash
sed -i "s|Exec=.*|Exec=/path/to/viewskater-egui.AppImage %f|" \
    ~/.local/share/applications/viewskater-egui.desktop
```
