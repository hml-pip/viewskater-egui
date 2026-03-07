use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use eframe::egui;

const DEFAULT_CACHE_COUNT: usize = 5;

pub struct DecodeResult {
    pub file_index: usize,
    pub image: Option<egui::ColorImage>,
    pub decode_ms: f64,
}

/// Sliding window cache that preloads neighboring images in background threads.
///
/// The window has `cache_count * 2 + 1` slots. `first_file_index` tracks which
/// file index slot 0 maps to. The current image sits at slot
/// `current_index - first_file_index`, ideally at the center (`cache_count`),
/// but off-center near directory boundaries.
pub struct SlidingWindowCache {
    slots: VecDeque<Option<egui::TextureHandle>>,
    first_file_index: usize,
    cache_count: usize,

    tx: mpsc::Sender<DecodeResult>,
    rx: mpsc::Receiver<DecodeResult>,
    in_flight: HashSet<usize>,

    ctx: egui::Context,
}

impl SlidingWindowCache {
    pub fn new(ctx: &egui::Context) -> Self {
        let cache_count = DEFAULT_CACHE_COUNT;
        let cache_size = cache_count * 2 + 1;
        let (tx, rx) = mpsc::channel();

        Self {
            slots: VecDeque::from(vec![None; cache_size]),
            first_file_index: 0,
            cache_count,
            tx,
            rx,
            in_flight: HashSet::new(),
            ctx: ctx.clone(),
        }
    }

    fn cache_size(&self) -> usize {
        self.cache_count * 2 + 1
    }

    /// Initialize the cache centered on `center_index`.
    /// Synchronously decodes the center image, spawns background loads for neighbors.
    pub fn initialize(&mut self, center_index: usize, image_paths: &[PathBuf]) {
        let num_files = image_paths.len();
        if num_files == 0 {
            return;
        }

        // Drain any pending results from previous window
        while self.rx.try_recv().is_ok() {}
        self.in_flight.clear();

        let cache_size = self.cache_size();

        // Position the window so center_index is at slot cache_count (center),
        // clamped to valid range
        let max_first = num_files.saturating_sub(cache_size);
        self.first_file_index = (center_index.saturating_sub(self.cache_count)).min(max_first);

        // Clear all slots
        self.slots.clear();
        self.slots.resize(cache_size, None);

        // Synchronously decode the center image
        let center_slot = center_index - self.first_file_index;
        if let Some(tex) = Self::decode_sync(&image_paths[center_index], &self.ctx) {
            self.slots[center_slot] = Some(tex);
        }

        // Spawn background loads for all other valid slots
        for i in 0..cache_size {
            if i == center_slot {
                continue;
            }
            let file_index = self.first_file_index + i;
            if file_index < num_files {
                self.spawn_load(file_index, &image_paths[file_index]);
            }
        }
    }

    /// Poll for completed background decodes and upload textures.
    /// Call this every frame from `update()`.
    pub fn poll(&mut self, image_paths: &[PathBuf]) {
        while let Ok(result) = self.rx.try_recv() {
            self.in_flight.remove(&result.file_index);

            let slot_idx = self.slot_index_for(result.file_index);
            if let Some(slot_idx) = slot_idx {
                if let Some(color_image) = result.image {
                    let name = image_paths
                        .get(result.file_index)
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let texture = self.ctx.load_texture(
                        name,
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.slots[slot_idx] = Some(texture);
                }
            }
            // else: stale result for index outside current window, drop
        }
    }

    /// Shift the cache window for forward navigation.
    /// Returns the TextureHandle for the new current image, or None on cache miss.
    pub fn navigate_forward(
        &mut self,
        new_index: usize,
        image_paths: &[PathBuf],
    ) -> Option<egui::TextureHandle> {
        let num_files = image_paths.len();
        let current_slot = new_index - self.first_file_index;

        if current_slot > self.cache_count {
            // Shift window right
            self.slots.pop_front();
            self.slots.push_back(None);
            self.first_file_index += 1;

            // Spawn load for new rightmost slot
            let new_file_index = self.first_file_index + self.cache_size() - 1;
            if new_file_index < num_files {
                self.spawn_load(new_file_index, &image_paths[new_file_index]);
            }
        }

        self.current_texture_for(new_index)
    }

    /// Shift the cache window for backward navigation.
    /// Returns the TextureHandle for the new current image, or None on cache miss.
    pub fn navigate_backward(
        &mut self,
        new_index: usize,
        image_paths: &[PathBuf],
    ) -> Option<egui::TextureHandle> {
        let current_slot = new_index - self.first_file_index;

        if current_slot < self.cache_count && self.first_file_index > 0 {
            // Shift window left
            self.slots.pop_back();
            self.slots.push_front(None);
            self.first_file_index -= 1;

            // Spawn load for new leftmost slot
            self.spawn_load(self.first_file_index, &image_paths[self.first_file_index]);
        }

        self.current_texture_for(new_index)
    }

    /// Rebuild cache around a new position (slider release, Home/End).
    pub fn jump_to(&mut self, new_index: usize, image_paths: &[PathBuf]) {
        self.initialize(new_index, image_paths);
    }

    /// Get the TextureHandle for a given file index, if cached.
    pub fn current_texture_for(&self, file_index: usize) -> Option<egui::TextureHandle> {
        let slot_idx = file_index.checked_sub(self.first_file_index)?;
        self.slots.get(slot_idx).and_then(|opt| opt.clone())
    }

    /// Find which slot (if any) holds the given file index.
    fn slot_index_for(&self, file_index: usize) -> Option<usize> {
        if file_index < self.first_file_index {
            return None;
        }
        let idx = file_index - self.first_file_index;
        if idx < self.slots.len() {
            Some(idx)
        } else {
            None
        }
    }

    /// Spawn a background thread to decode an image.
    fn spawn_load(&mut self, file_index: usize, path: &PathBuf) {
        if self.in_flight.contains(&file_index) {
            return;
        }
        self.in_flight.insert(file_index);

        let path = path.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let start = Instant::now();
            let image = match image::open(&path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    Some(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
                }
                Err(e) => {
                    log::warn!("Background decode failed for {}: {}", path.display(), e);
                    None
                }
            };
            let decode_ms = start.elapsed().as_secs_f64() * 1000.0;
            let _ = tx.send(DecodeResult {
                file_index,
                image,
                decode_ms,
            });
            ctx.request_repaint();
        });
    }

    /// Synchronously decode an image and upload as a texture.
    fn decode_sync(path: &PathBuf, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
            }
            Err(e) => {
                log::error!("Failed to decode {}: {}", path.display(), e);
                None
            }
        }
    }
}
