use eframe::egui;

use crate::app::editor::{FontEditor, PaintMode};
use hangul_syllable::{GlyphKey, PixelGlyph};

impl FontEditor {
    pub fn render_local_pixel_editor(&mut self, ui: &mut egui::Ui, key: GlyphKey) {
        let avail = ui.available_size();
        let max_px = avail.x.min(avail.y).max(64.0);
        let grid_step = self
            .drawing
            .grid_size
            .min(max_px / self.project.canvas_w.max(self.project.canvas_h) as f32)
            .max(1.0);
        let canvas_w = self.project.canvas_w;
        let canvas_h = self.project.canvas_h;

        let desired_size = egui::vec2(canvas_w as f32 * grid_step, canvas_h as f32 * grid_step);

        ui.allocate_ui_with_layout(desired_size, egui::Layout::top_down(egui::Align::Center), |ui| {
            self.render_local_pixel_editor_fixed(ui, &key, canvas_w, canvas_h, grid_step);
        });
    }

    fn render_local_pixel_editor_fixed(&mut self, ui: &mut egui::Ui, key: &GlyphKey, canvas_w: i32, canvas_h: i32, grid_step: f32) {
        let desired_size = egui::vec2(canvas_w as f32 * grid_step, canvas_h as f32 * grid_step);
        let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::click_and_drag());
        let rect = response.rect;
        let dark = ui.visuals().dark_mode;

        draw_editor_background(&painter, rect, dark);

        let changed = self.handle_paint_input(ui, &response, key, canvas_w, canvas_h, grid_step);
        if changed {
            self.invalidate_render_caches();
            ui.ctx().request_repaint();
        }

        draw_grid(&painter, rect, canvas_w, canvas_h, grid_step, dark);

        if let Some(glyph) = self.project.store.get(key) {
            draw_pixels(&painter, rect, grid_step, canvas_w, canvas_h, glyph, dark);
        }

        if response.hovered() {
            self.drawing.active_edit_key = Some(key.clone());
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
        }
    }

    fn handle_paint_input(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        key: &GlyphKey,
        canvas_w: i32,
        canvas_h: i32,
        grid_step: f32,
    ) -> bool {
        let pointer_pos = response.interact_pointer_pos();
        let (pressed_l, down_l, released_l, pressed_r, down_r, released_r) = ui.ctx().input(|i| {
            (
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_down(egui::PointerButton::Primary),
                i.pointer.button_released(egui::PointerButton::Primary),
                i.pointer.button_pressed(egui::PointerButton::Secondary),
                i.pointer.button_down(egui::PointerButton::Secondary),
                i.pointer.button_released(egui::PointerButton::Secondary),
            )
        });

        if response.hovered() {
            if pressed_l {
                let mode = if self.drawing.pen_toggle_mode {
                    let toggled = pointer_pos.and_then(|pos| {
                        let local = pos - response.rect.min;
                        let gx = (local.x / grid_step).floor() as i32;
                        let gy = (local.y / grid_step).floor() as i32;
                        if gx >= 0 && gy >= 0 && gx < canvas_w && gy < canvas_h {
                            let is_set = self.project.store.get(key).is_some_and(|g| g.pixels.contains(&(gx, gy)));
                            Some(if is_set { PaintMode::Erase } else { PaintMode::Draw })
                        } else {
                            None
                        }
                    });
                    toggled.unwrap_or(PaintMode::Draw)
                } else {
                    PaintMode::Draw
                };
                self.drawing.paint_mode = Some(mode);
                self.drawing.last_paint_cell = None;
                self.drawing.stroke_start = Some((key.clone(), self.get_glyph_pixels(key)));
            } else if pressed_r && !self.drawing.pen_toggle_mode {
                self.drawing.paint_mode = Some(PaintMode::Erase);
                self.drawing.last_paint_cell = None;
                self.drawing.stroke_start = Some((key.clone(), self.get_glyph_pixels(key)));
            }
        }

        let ends_stroke = released_l || (!self.drawing.pen_toggle_mode && released_r);
        if ends_stroke {
            if let Some((snap_key, snapshot)) = self.drawing.stroke_start.take() {
                let changed = self.project.store.get(&snap_key).is_some_and(|g| g.pixels != snapshot);
                if changed {
                    self.drawing.undo_stack.push((snap_key, snapshot));
                    if self.drawing.undo_stack.len() > 50 {
                        self.drawing.undo_stack.remove(0);
                    }
                    self.drawing.redo_stack.clear();
                }
            }
            self.drawing.paint_mode = None;
            self.drawing.last_paint_cell = None;
        }

        let is_painting = match self.drawing.paint_mode {
            Some(PaintMode::Draw) => down_l,
            Some(PaintMode::Erase) => {
                if self.drawing.pen_toggle_mode {
                    down_l
                } else {
                    down_r
                }
            }
            None => false,
        };

        if is_painting && let Some(pos) = pointer_pos {
            let local = pos - response.rect.min;
            let gx = (local.x / grid_step).floor() as i32;
            let gy = (local.y / grid_step).floor() as i32;

            if gx >= 0 && gy >= 0 && gx < canvas_w && gy < canvas_h && self.drawing.last_paint_cell != Some((gx, gy)) {
                self.drawing.last_paint_cell = Some((gx, gy));
                if let Some(glyph) = self.project.store.get_mut(key) {
                    match self.drawing.paint_mode {
                        Some(PaintMode::Draw) => glyph.set(gx, gy),
                        Some(PaintMode::Erase) => glyph.clear(gx, gy),
                        None => {}
                    }
                }
                return true;
            }
        }

        false
    }
}

fn draw_editor_background(painter: &egui::Painter, rect: egui::Rect, dark: bool) {
    painter.rect_filled(
        rect,
        0.0,
        if dark {
            egui::Color32::BLACK
        } else {
            egui::Color32::from_gray(245)
        },
    );
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::GRAY), egui::StrokeKind::Middle);
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect, canvas_w: i32, canvas_h: i32, grid_step: f32, dark: bool) {
    let stroke = egui::Stroke::new(
        1.0,
        if dark {
            egui::Color32::from_gray(30)
        } else {
            egui::Color32::from_gray(200)
        },
    );
    for i in 0..=canvas_w {
        let x = rect.min.x + i as f32 * grid_step;
        painter.line_segment([egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)], stroke);
    }
    for i in 0..=canvas_h {
        let y = rect.min.y + i as f32 * grid_step;
        painter.line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], stroke);
    }
}

fn draw_pixels(painter: &egui::Painter, rect: egui::Rect, grid_step: f32, canvas_w: i32, canvas_h: i32, glyph: &PixelGlyph, dark: bool) {
    let fill = if dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
    for &(px, py) in &glyph.pixels {
        if px < 0 || py < 0 || px >= canvas_w || py >= canvas_h {
            continue;
        }
        let pos = rect.min + egui::vec2(px as f32 * grid_step, py as f32 * grid_step);
        painter.rect_filled(egui::Rect::from_min_size(pos, egui::vec2(grid_step, grid_step)), 0.0, fill);
    }
}
