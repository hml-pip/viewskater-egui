# viewskater-egui

An egui-based image viewer — a simplified reimplementation of [viewskater](https://github.com/ggand0/viewskater)'s core rendering and navigation. Built as a framework evaluation and foundation for a LeRobot dataset curation tool.

## Features

- Open image directories via CLI argument or drag-and-drop
- Arrow key navigation with natural sort ordering
- Scroll-to-zoom centered on cursor, click-drag to pan, double-click to reset
- Navigation slider for jumping to any position
- Supports jpg, png, bmp, webp, gif, tiff, qoi, tga

## Usage

```bash
# View a directory of images
cargo run --profile opt-dev -- /path/to/images/

# View a specific image (opens its parent directory)
cargo run --profile opt-dev -- /path/to/image.jpg

# Launch empty and drag-and-drop a folder onto the window
cargo run --profile opt-dev
```

Set `RUST_LOG=viewskater_egui=debug` for debug logging.

## Controls

| Input | Action |
|---|---|
| Left / Right arrow | Previous / next image |
| Home / End | First / last image |
| Scroll wheel | Zoom (centered on cursor) |
| Click + drag | Pan |
| Double-click | Reset zoom and pan |
| Slider | Jump to position |