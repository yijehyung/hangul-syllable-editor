use eframe::egui;

use crate::app::editor::FontEditor;
use hangul_syllable::HangulComponent;
use hangul_syllable::core::{
    hangul::{allowed_chars_for_target, decompose_hangul, get_jamo_char},
    render::compose_pixels,
};

fn is_hangul_syllable(ch: char) -> bool {
    let cp = ch as u32;
    (0xAC00..=0xD7A3).contains(&cp)
}

// Jamo 블록 초성 (U+1100~U+115E, U+A960~U+A97C) 또는 호환 자모 자음 (U+3131~U+314E)
fn is_cho(ch: char) -> bool {
    let cp = ch as u32;
    (0x1100..=0x115E).contains(&cp) || (0xA960..=0xA97C).contains(&cp) || (0x3131..=0x314E).contains(&cp)
}

// Jamo 블록 중성 (U+1161~U+11A7, U+D7B0~U+D7C6) 또는 호환 자모 모음 (U+314F~U+3163)
fn is_jung(ch: char) -> bool {
    let cp = ch as u32;
    (0x1161..=0x11A7).contains(&cp) || (0xD7B0..=0xD7C6).contains(&cp) || (0x314F..=0x3163).contains(&cp)
}

// Jamo 블록 종성 (U+11A8~U+11FF, U+D7CB~U+D7FB) 또는 호환 자모 자음 (문맥상 종성 위치)
fn is_jong_block(ch: char) -> bool {
    let cp = ch as u32;
    (0x11A8..=0x11FF).contains(&cp) || (0xD7CB..=0xD7FB).contains(&cp)
}

fn is_compat_consonant(ch: char) -> bool {
    let cp = ch as u32;
    (0x3131..=0x314E).contains(&cp)
}

fn normalize_cho(ch: char) -> char {
    let cp = ch as u32;
    if (0x1100..=0x1112).contains(&cp) {
        let i = (cp - 0x1100) as usize;
        allowed_chars_for_target(HangulComponent::Cho).get(i).copied().unwrap_or(ch)
    } else {
        ch
    }
}

fn normalize_jung(ch: char) -> char {
    let cp = ch as u32;
    if (0x1161..=0x1175).contains(&cp) {
        let i = (cp - 0x1161) as usize;
        allowed_chars_for_target(HangulComponent::Jung).get(i).copied().unwrap_or(ch)
    } else {
        ch
    }
}

fn normalize_jong(ch: char) -> char {
    let cp = ch as u32;
    if (0x11A8..=0x11C2).contains(&cp) {
        let i = (cp - 0x11A8) as usize;
        allowed_chars_for_target(HangulComponent::Jong).get(i).copied().unwrap_or(ch)
    } else {
        ch
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_pixels(
    painter: &egui::Painter,
    pixels: &[hangul_syllable::core::render::ComposedPixel],
    pen_x: f32,
    pen_y: f32,
    zoom: f32,
    shadow_ofs: &[(i32, i32)],
    fg: egui::Color32,
    shadow_color: egui::Color32,
    shadow_enabled: bool,
) {
    if shadow_enabled && !shadow_ofs.is_empty() {
        for (dx, dy) in shadow_ofs {
            for px in pixels {
                let r = egui::Rect::from_min_size(
                    egui::pos2(pen_x + ((px.x + dx) as f32) * zoom, pen_y + ((px.y + dy) as f32) * zoom),
                    egui::vec2(zoom, zoom),
                );
                painter.rect_filled(r, 0.0, shadow_color);
            }
        }
    }
    for px in pixels {
        let r = egui::Rect::from_min_size(
            egui::pos2(pen_x + px.x as f32 * zoom, pen_y + px.y as f32 * zoom),
            egui::vec2(zoom, zoom),
        );
        painter.rect_filled(r, 0.0, fg);
    }
}

fn shadow_offsets(px: i32, dirs: &[bool; 8]) -> Vec<(i32, i32)> {
    if px <= 0 {
        return vec![];
    }
    let dirs_vec: [(i32, i32); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];

    let mut out = Vec::new();
    for (i, (dx, dy)) in dirs_vec.iter().copied().enumerate() {
        if !dirs[i] {
            continue;
        }
        for step in 1..=px {
            out.push((dx * step, dy * step));
        }
    }
    out
}

impl FontEditor {
    pub fn ui_type_test_mode(&mut self, ui: &mut egui::Ui) {
        use crate::app::ui_widgets::sub_panel_tabs;

        if ui.available_width() < crate::app::editor::NARROW_WIDTH {
            let s = crate::i18n::t(self.lang);
            sub_panel_tabs(ui, &mut self.narrow_tt_sub, &[s.type_test.settings, s.common.test]);
            ui.separator();
            match self.narrow_tt_sub {
                0 => self.render_typetest_settings_content(ui),
                _ => self.render_typetest_canvas_content(ui),
            }
            return;
        }

        let s = crate::i18n::t(self.lang);
        egui::Panel::left("typetest_controls")
            .min_size(220.0)
            .max_size(360.0)
            .default_size(260.0)
            .show_inside(ui, |ui| {
                ui.heading(s.type_test.settings);
                ui.separator();
                self.render_typetest_settings_content(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_typetest_canvas_content(ui);
        });
    }

    fn render_typetest_settings_content(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(s.type_test.text_label);
            ui.add(
                egui::TextEdit::multiline(&mut self.type_test.text)
                    .desired_rows(6)
                    .hint_text(s.type_test.text_hint)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            ui.separator();

            egui::Grid::new("typetest_settings_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(s.type_test.zoom);
                    ui.add(egui::Slider::new(&mut self.type_test.zoom, 1.0..=16.0).fixed_decimals(1));
                    ui.end_row();

                    ui.label(s.type_test.letter_spacing);
                    ui.add(egui::DragValue::new(&mut self.type_test.letter_spacing).range(0.0..=40.0));
                    ui.end_row();

                    ui.label(s.type_test.line_spacing);
                    ui.add(egui::DragValue::new(&mut self.type_test.line_spacing).range(0.0..=80.0));
                    ui.end_row();

                    ui.label(s.type_test.space_px);
                    ui.add(egui::DragValue::new(&mut self.type_test.space_px).range(0..=40));
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.separator();

            egui::Grid::new("typetest_color_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(s.type_test.text_color);
                    ui.color_edit_button_srgba(&mut self.type_test.fg);
                    ui.end_row();

                    ui.label(s.type_test.bg_color);
                    ui.color_edit_button_srgba(&mut self.type_test.bg);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.separator();

            ui.checkbox(&mut self.type_test.shadow_enabled, s.type_test.shadow);
            ui.add_enabled_ui(self.type_test.shadow_enabled, |ui| {
                egui::Grid::new("typetest_shadow_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(s.type_test.shadow_color);
                        ui.color_edit_button_srgba(&mut self.type_test.shadow_color);
                        ui.end_row();

                        ui.label(s.type_test.shadow_px);
                        ui.add(egui::DragValue::new(&mut self.type_test.shadow_px).range(0..=32));
                        ui.end_row();
                    });

                ui.label(s.type_test.direction);
                // NW(7) N(0) NE(1) / W(6) · E(2) / SW(5) S(4) SE(3)
                let grid_cells: [Option<usize>; 9] = [Some(7), Some(0), Some(1), Some(6), None, Some(2), Some(5), Some(4), Some(3)];
                let labels = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
                egui::Grid::new("shadow_dir_grid")
                    .num_columns(3)
                    .spacing([2.0, 2.0])
                    .show(ui, |ui| {
                        for (cell_i, cell) in grid_cells.iter().enumerate() {
                            if let Some(idx) = cell {
                                let on = self.type_test.shadow_dirs[*idx];
                                if ui
                                    .add(egui::Button::new(labels[*idx]).selected(on).min_size(egui::vec2(24.0, 24.0)))
                                    .clicked()
                                {
                                    self.type_test.shadow_dirs[*idx] = !on;
                                }
                            } else {
                                ui.add_enabled(false, egui::Button::new("·").min_size(egui::vec2(24.0, 24.0)));
                            }
                            if (cell_i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.small_button(s.type_test.all).clicked() {
                        self.type_test.shadow_dirs = [true; 8];
                    }
                    if ui.small_button(s.type_test.none).clicked() {
                        self.type_test.shadow_dirs = [false; 8];
                    }
                });
            });
        });
    }

    fn render_typetest_canvas_content(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width().max(1.0);
        let h = ui.available_height().max(1.0);
        let (resp, painter) = ui.allocate_painter(egui::vec2(w, h), egui::Sense::hover());
        let rect = resp.rect;

        painter.rect_filled(rect, 6.0, self.type_test.bg);
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            egui::StrokeKind::Middle,
        );

        let pad = 10.0;
        let mut pen_x = rect.min.x + pad;
        let mut pen_y = rect.min.y + pad;

        let zoom = self.type_test.zoom.max(1.0);
        let px_advance = (self.project.canvas_w as f32 + self.type_test.letter_spacing) * zoom;
        let line_height = (self.project.canvas_h as f32 + self.type_test.line_spacing) * zoom;

        let font_id = egui::FontId::new(self.type_test.font_size, egui::FontFamily::Proportional);

        let shadow_ofs = if self.type_test.shadow_enabled {
            shadow_offsets(self.type_test.shadow_px, &self.type_test.shadow_dirs)
        } else {
            vec![]
        };

        if self.type_test.cache_em != (self.project.canvas_w, self.project.canvas_h) {
            self.type_test.pixel_cache.clear();
            self.type_test.cache_em = (self.project.canvas_w, self.project.canvas_h);
        }

        let chars: Vec<char> = self.type_test.text.chars().collect();
        let mut i = 0;
        'outer: while i < chars.len() {
            let ch = chars[i];

            if ch == '\n' {
                pen_x = rect.min.x + pad;
                pen_y += line_height;
                i += 1;
                continue;
            }

            if ch == ' ' {
                let space_advance = (self.type_test.space_px as f32 + self.type_test.letter_spacing) * zoom;
                pen_x += space_advance;
                if pen_x > rect.max.x - pad {
                    pen_x = rect.min.x + pad;
                    pen_y += line_height;
                }
                i += 1;
                continue;
            }

            if pen_y + line_height > rect.max.y - pad {
                break;
            }

            if is_hangul_syllable(ch) {
                if pen_x + px_advance > rect.max.x - pad {
                    pen_x = rect.min.x + pad;
                    pen_y += line_height;
                    if pen_y + line_height > rect.max.y - pad {
                        break;
                    }
                }
                if !self.type_test.pixel_cache.contains_key(&ch) {
                    let pixels = self
                        .project
                        .engine
                        .layout_char(&self.project.store, ch, decompose_hangul, get_jamo_char)
                        .map(|res| compose_pixels(&self.project.store, &res, self.project.canvas_w, self.project.canvas_h))
                        .unwrap_or_default();
                    self.type_test.pixel_cache.insert(ch, pixels);
                }
                let pixels = &self.type_test.pixel_cache[&ch];
                draw_pixels(
                    &painter,
                    pixels,
                    pen_x,
                    pen_y,
                    zoom,
                    &shadow_ofs,
                    self.type_test.fg,
                    self.type_test.shadow_color,
                    self.type_test.shadow_enabled,
                );
                pen_x += px_advance;
                i += 1;
                continue;
            }

            // 자모 시퀀스 — 초성 감지 후 중성/종성 lookahead
            if is_cho(ch) {
                let next1 = chars.get(i + 1).copied();
                if let Some(jung_ch) = next1.filter(|&c| is_jung(c)) {
                    let next2 = chars.get(i + 2).copied();
                    let next3 = chars.get(i + 3).copied();
                    let jong_ch = next2.and_then(|c| {
                        if is_jong_block(c) {
                            Some(c)
                        } else if is_compat_consonant(c) {
                            if next3.is_none_or(|n| !is_jung(n)) { Some(c) } else { None }
                        } else {
                            None
                        }
                    });
                    let consumed = 2 + jong_ch.map_or(0, |_| 1);
                    let cho_n = normalize_cho(ch);
                    let jung_n = normalize_jung(jung_ch);
                    let jong_n = jong_ch.map(normalize_jong);
                    let triplet = (cho_n, jung_n, jong_n);

                    if pen_x + px_advance > rect.max.x - pad {
                        pen_x = rect.min.x + pad;
                        pen_y += line_height;
                        if pen_y + line_height > rect.max.y - pad {
                            break 'outer;
                        }
                    }
                    if !self.type_test.triplet_cache.contains_key(&triplet) {
                        let pixels = self
                            .project
                            .engine
                            .layout_jamo(&self.project.store, cho_n, jung_n, jong_n)
                            .map(|res| compose_pixels(&self.project.store, &res, self.project.canvas_w, self.project.canvas_h))
                            .unwrap_or_default();
                        self.type_test.triplet_cache.insert(triplet, pixels);
                    }
                    let pixels = &self.type_test.triplet_cache[&triplet];
                    draw_pixels(
                        &painter,
                        pixels,
                        pen_x,
                        pen_y,
                        zoom,
                        &shadow_ofs,
                        self.type_test.fg,
                        self.type_test.shadow_color,
                        self.type_test.shadow_enabled,
                    );
                    pen_x += px_advance;
                    i += consumed;
                    continue;
                }
            }

            let galley = painter.layout_no_wrap(ch.to_string(), font_id.clone(), self.type_test.fg);
            let gw = galley.size().x;
            if pen_x + gw > rect.max.x - pad {
                pen_x = rect.min.x + pad;
                pen_y += line_height;
                if pen_y + line_height > rect.max.y - pad {
                    break;
                }
            }
            if self.type_test.shadow_enabled && !shadow_ofs.is_empty() {
                for (dx, dy) in &shadow_ofs {
                    painter.galley(
                        egui::pos2(pen_x + (*dx as f32) * zoom, pen_y + (*dy as f32) * zoom),
                        galley.clone(),
                        self.type_test.shadow_color,
                    );
                }
            }
            painter.galley(egui::pos2(pen_x, pen_y), galley, self.type_test.fg);
            pen_x += gw + self.type_test.letter_spacing * zoom;
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_syllable_boundaries() {
        assert!(is_hangul_syllable('가'));
        assert!(is_hangul_syllable('힣'));
        assert!(!is_hangul_syllable('힤'));
        assert!(!is_hangul_syllable('ㄱ'));
    }

    #[test]
    fn cho_ranges() {
        assert!(is_cho('ㄱ')); // compat consonant U+3131
        assert!(is_cho('\u{1100}')); // jamo block cho start
        assert!(is_cho('\u{115E}')); // jamo block cho end
        assert!(is_cho('\u{A960}')); // extended-A cho start
        assert!(!is_cho('ㅏ'));
        assert!(!is_cho('가'));
    }

    #[test]
    fn jung_ranges() {
        assert!(is_jung('ㅏ')); // compat vowel U+314F
        assert!(is_jung('\u{1161}')); // jamo block jung start
        assert!(is_jung('\u{11A7}')); // jamo block jung end
        assert!(is_jung('\u{D7B0}')); // extended-B jung start
        assert!(!is_jung('ㄱ'));
    }

    #[test]
    fn jong_block_ranges() {
        assert!(is_jong_block('\u{11A8}')); // jamo block jong start
        assert!(is_jong_block('\u{11FF}')); // jamo block jong end
        assert!(is_jong_block('\u{D7CB}')); // extended-B jong start
        assert!(is_jong_block('\u{D7FB}')); // extended-B jong end
        assert!(!is_jong_block('ㄱ')); // compat consonant — not in jong block
    }
}
