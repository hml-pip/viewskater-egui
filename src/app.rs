use std::path::PathBuf;

use eframe::egui;

use crate::pane::PaneState;
use crate::perf;

pub struct App {
    panes: Vec<PaneState>,
    perf: perf::ImagePerfTracker,
    divider_fraction: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        let mut app = Self {
            panes: vec![PaneState::new()],
            perf: perf::ImagePerfTracker::new(),
            divider_fraction: 0.5,
        };

        if !paths.is_empty() {
            app.panes[0].open_path(&paths[0], &cc.egui_ctx);
        }
        if paths.len() >= 2 {
            let mut pane1 = PaneState::new();
            pane1.open_path(&paths[1], &cc.egui_ctx);
            app.panes.push(pane1);
        }

        if app.panes[0].current_texture.is_some() {
            app.perf.record_image_load(0.0);
        }

        app
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let (home, end, nav_right_held, nav_left_held, toggle_dual, set_single, set_dual) =
            ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Home),
                    i.key_pressed(egui::Key::End),
                    i.key_down(egui::Key::ArrowRight) || i.key_down(egui::Key::D),
                    i.key_down(egui::Key::ArrowLeft) || i.key_down(egui::Key::A),
                    i.key_pressed(egui::Key::Tab),
                    i.key_pressed(egui::Key::Num1) && i.modifiers.command,
                    i.key_pressed(egui::Key::Num2) && i.modifiers.command,
                )
            });

        if set_single && self.panes.len() >= 2 {
            self.panes.truncate(1);
            return;
        }
        if set_dual && self.panes.len() == 1 {
            let mut pane = PaneState::new();
            if !self.panes[0].image_paths.is_empty() {
                if let Some(dir) = self.panes[0].image_paths[0].parent() {
                    pane.open_path(dir, ctx);
                    pane.jump_to(self.panes[0].current_index, ctx);
                }
            }
            self.panes.push(pane);
            return;
        }

        if toggle_dual {
            if self.panes.len() >= 2 {
                self.panes.truncate(1);
            } else if !self.panes.is_empty() {
                let mut pane = PaneState::new();
                if !self.panes[0].image_paths.is_empty() {
                    if let Some(dir) = self.panes[0].image_paths[0].parent() {
                        pane.open_path(dir, ctx);
                        pane.jump_to(self.panes[0].current_index, ctx);
                    }
                }
                self.panes.push(pane);
            }
            return;
        }

        if home {
            for pane in &mut self.panes {
                pane.jump_to(0, ctx);
            }
            self.perf.record_image_load(0.0);
        } else if end {
            for pane in &mut self.panes {
                let last = pane.image_paths.len().saturating_sub(1);
                pane.jump_to(last, ctx);
            }
            self.perf.record_image_load(0.0);
        } else if nav_right_held {
            // Only advance if ALL panes have the next image cached (synced nav)
            let all_ready = self.panes.iter().all(|p| p.is_next_cached(1));
            if all_ready {
                // fold instead of any() to avoid short-circuit — call navigate on every pane
                let any_advanced = self.panes.iter_mut().fold(false, |acc, p| p.navigate(1) || acc);
                if any_advanced {
                    self.perf.record_image_load(0.0);
                }
            }
            let any_can = self.panes.iter().any(|p| p.can_navigate_forward());
            if any_can {
                ctx.request_repaint();
            }
        } else if nav_left_held {
            let all_ready = self.panes.iter().all(|p| p.is_next_cached(-1));
            if all_ready {
                let any_advanced = self.panes.iter_mut().fold(false, |acc, p| p.navigate(-1) || acc);
                if any_advanced {
                    self.perf.record_image_load(0.0);
                }
            }
            let any_can = self.panes.iter().any(|p| p.can_navigate_backward());
            if any_can {
                ctx.request_repaint();
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<egui::DroppedFile> = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.first() {
            if let Some(path) = &file.path {
                if self.panes.len() >= 2 {
                    let hover = ctx.input(|i| i.pointer.hover_pos());
                    let latest = ctx.input(|i| i.pointer.latest_pos());
                    let screen = ctx.screen_rect();
                    let divider_x =
                        screen.min.x + screen.width() * self.divider_fraction;
                    log::debug!(
                        "DnD drop: hover_pos={:?}, latest_pos={:?}, screen={:?}, divider_x={}, fraction={}",
                        hover, latest, screen, divider_x, self.divider_fraction
                    );
                    let target = hover
                        .or(latest)
                        .map(|pos| {
                            log::debug!("DnD using pos x={}, divider_x={} → pane {}", pos.x, divider_x, if pos.x < divider_x { 0 } else { 1 });
                            if pos.x < divider_x { 0 } else { 1 }
                        })
                        .unwrap_or(0);
                    self.panes[target].open_path(path, ctx);
                } else {
                    self.panes[0].open_path(path, ctx);
                }
            }
        }
    }

    fn update_title(&self, ctx: &egui::Context) {
        let parts: Vec<String> = self
            .panes
            .iter()
            .filter_map(|pane| {
                pane.image_paths.get(pane.current_index).map(|path| {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    format!("{} ({}/{})", name, pane.current_index + 1, pane.image_paths.len())
                })
            })
            .collect();

        let title = if parts.is_empty() {
            "viewskater-egui".to_string()
        } else {
            format!("{} - viewskater-egui", parts.join(" | "))
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        let max_images = self
            .panes
            .iter()
            .map(|p| p.image_paths.len())
            .max()
            .unwrap_or(0);
        if max_images <= 1 {
            return;
        }

        let current_idx = self.panes.first().map_or(0, |p| p.current_index);
        let mut slider_target = None;
        let mut slider_released = false;

        egui::TopBottomPanel::bottom("nav").show(ctx, |ui| {
            let label_text = format!("{} / {}", current_idx + 1, max_images);

            ui.horizontal(|ui| {
                let mut idx = current_idx;
                let max = max_images - 1;

                // Measure label width so slider can fill the rest
                let label_galley = ui.fonts(|f| {
                    f.layout_no_wrap(
                        label_text.clone(),
                        egui::FontId::default(),
                        egui::Color32::WHITE,
                    )
                });
                let label_width = label_galley.size().x + ui.spacing().item_spacing.x * 2.0;

                // Override slider_width to fill available space minus label
                ui.spacing_mut().slider_width = ui.available_width() - label_width;

                let response =
                    ui.add(egui::Slider::new(&mut idx, 0..=max).show_value(false));
                if response.changed() {
                    slider_target = Some(idx);
                }
                if response.drag_stopped() {
                    slider_released = true;
                }
                ui.label(label_text);
            });
        });

        if let Some(idx) = slider_target {
            for pane in &mut self.panes {
                let clamped = idx.min(pane.image_paths.len().saturating_sub(1));
                if clamped != pane.current_index {
                    pane.current_index = clamped;
                    pane.zoom = 1.0;
                    pane.pan = egui::Vec2::ZERO;

                    // Try the sliding window cache first (free if already cached)
                    let found_in_cache = pane
                        .cache
                        .as_ref()
                        .and_then(|c| c.current_texture_for(clamped));

                    if let Some(tex) = found_in_cache {
                        pane.current_texture = Some(tex);
                        self.perf.record_image_load(0.0);
                    } else if let Some(loader) = &mut pane.slider_loader {
                        // Throttled sync decode — like iced, only decode when
                        // enough time has passed. Previous texture stays on screen.
                        if loader.should_load() {
                            pane.load_sync(ctx);
                            self.perf.record_image_load(0.0);
                        }
                    }
                }
            }
            ctx.request_repaint();
        }

        if slider_released {
            for pane in &mut self.panes {
                if let Some(loader) = &mut pane.slider_loader {
                    loader.cancel();
                }
                if let Some(cache) = &mut pane.cache {
                    cache.jump_to(pane.current_index, &pane.image_paths);
                    if let Some(t) = cache.current_texture_for(pane.current_index) {
                        pane.current_texture = Some(t);
                    }
                }
            }
        }
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_gray(20)))
            .show(ctx, |ui| {
                if self.panes.len() <= 1 {
                    if let Some(pane) = self.panes.first_mut() {
                        pane.show_content(ui);
                    }
                } else {
                    let available = ui.available_rect_before_wrap();
                    let divider_w = 4.0;
                    let grab_w = 12.0; // wider hit area for easy grabbing
                    let left_w = (available.width() - divider_w) * self.divider_fraction;

                    let left_rect = egui::Rect::from_min_size(
                        available.min,
                        egui::vec2(left_w, available.height()),
                    );
                    let right_rect = egui::Rect::from_min_size(
                        egui::pos2(available.min.x + left_w + divider_w, available.min.y),
                        egui::vec2(available.width() - left_w - divider_w, available.height()),
                    );

                    // Divider interaction — wide grab area centered on the visual line
                    let divider_center_x = available.min.x + left_w + divider_w / 2.0;
                    let grab_rect = egui::Rect::from_center_size(
                        egui::pos2(divider_center_x, available.center().y),
                        egui::vec2(grab_w, available.height()),
                    );
                    let divider_response =
                        ui.allocate_rect(grab_rect, egui::Sense::click_and_drag());

                    if divider_response.dragged() {
                        let usable = available.width() - divider_w;
                        if usable > 0.0 {
                            let delta = divider_response.drag_delta().x;
                            self.divider_fraction =
                                (self.divider_fraction + delta / usable).clamp(0.1, 0.9);
                        }
                    }

                    // Double-click resets to 50/50
                    if divider_response.double_clicked() {
                        self.divider_fraction = 0.5;
                    }

                    // Resize cursor when hovering or dragging
                    if divider_response.hovered() || divider_response.dragged() {
                        ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }

                    // Visual divider line — highlighted when interacting
                    let divider_color = if divider_response.dragged() {
                        egui::Color32::from_gray(140)
                    } else if divider_response.hovered() {
                        egui::Color32::from_gray(100)
                    } else {
                        egui::Color32::from_gray(60)
                    };
                    ui.painter().vline(
                        divider_center_x,
                        available.y_range(),
                        egui::Stroke::new(divider_w, divider_color),
                    );

                    let (first, rest) = self.panes.split_at_mut(1);
                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(left_rect),
                        |ui| first[0].show_content(ui),
                    );
                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(right_rect),
                        |ui| rest[0].show_content(ui),
                    );
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for pane in &mut self.panes {
            pane.poll_cache();
        }

        self.handle_dropped_files(ctx);
        self.handle_keyboard(ctx);
        self.update_title(ctx);
        self.show_bottom_panel(ctx);
        self.show_central_panel(ctx);
        self.perf.show_overlay(ctx);

        if let Some(pane) = self.panes.first() {
            if let Some(cache) = &pane.cache {
                cache.show_debug_overlay(ctx, pane.current_index, pane.image_paths.len());
            }
        }
    }
}
