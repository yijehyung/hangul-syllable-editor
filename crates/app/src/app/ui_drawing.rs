pub mod cell_colors {
    use eframe::egui::Color32;
    pub fn bg_normal(dark: bool) -> Color32 {
        if dark { Color32::from_gray(20) } else { Color32::from_gray(230) }
    }
    pub fn bg_hovered(dark: bool) -> Color32 {
        if dark { Color32::from_gray(35) } else { Color32::from_gray(210) }
    }
    pub fn bg_selected(dark: bool) -> Color32 {
        if dark { Color32::from_gray(50) } else { Color32::from_gray(190) }
    }
    pub fn border_normal(dark: bool) -> Color32 {
        if dark { Color32::from_gray(60) } else { Color32::from_gray(160) }
    }
    pub fn border_selected(dark: bool) -> Color32 {
        if dark { Color32::WHITE } else { Color32::BLACK }
    }
}

use eframe::egui;

use crate::app::editor::FontEditor;
use hangul_syllable::LayoutResult;
use hangul_syllable::core::{
    groups::group_display_name,
    hangul::{decompose_hangul, get_jamo_char},
    render::compose_pixels,
    types::HangulComponent,
};

impl FontEditor {
    pub fn ui_drawing_mode(&mut self, ui: &mut egui::Ui) {
        use crate::app::ui_widgets::sub_panel_tabs;

        if ui.available_width() < crate::app::editor::NARROW_WIDTH {
            let s = crate::i18n::t(self.lang);
            sub_panel_tabs(ui, &mut self.narrow_draw_sub, &[s.common.list, s.common.edit]);
            ui.separator();
            match self.narrow_draw_sub {
                0 => {
                    ui.add_space(8.0);
                    self.ui_hangul_browser_sidebar(ui);
                }
                _ => self.render_drawing_edit_area(ui, true),
            }
            return;
        }

        egui::Panel::left("draw_side")
            .resizable(true)
            .default_size(330.0)
            .min_size(330.0)
            .max_size(520.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.ui_hangul_browser_sidebar(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_drawing_edit_area(ui, false);
        });
    }

    fn render_drawing_edit_area(&mut self, ui: &mut egui::Ui, force_part_tabs: bool) {
        let s = crate::i18n::t(self.lang);
        let layout = self.compute_layout();

        self.render_preview_and_info(ui, layout.as_ref());

        ui.add_space(6.0);
        ui.separator();

        let Some((cho_idx, jung_idx, jong_idx)) = decompose_hangul(self.selected_char) else {
            ui.add_space(6.0);
            ui.label(s.drawing.select_hangul);
            return;
        };

        let Some(res) = &layout else {
            ui.add_space(6.0);
            ui.label(s.drawing.layout_failed);
            return;
        };

        let cho_jamo = get_jamo_char(HangulComponent::Cho, cho_idx);
        let jung_jamo = get_jamo_char(HangulComponent::Jung, jung_idx);
        let jong_jamo = get_jamo_char(HangulComponent::Jong, jong_idx);

        let cho_gid = Some(res.cho.group_id.clone());
        let jung_gid = Some(res.jung.group_id.clone());
        let jong_gid = if jong_idx == 0 {
            None
        } else {
            res.jong.as_ref().map(|j| j.group_id.clone())
        };

        let min_col_w = self.project.canvas_w.max(self.project.canvas_h) as f32 * 8.0 + 20.0;
        let use_three_cols = !force_part_tabs && ui.available_width() >= min_col_w * 3.0;

        if use_three_cols {
            ui.columns(3, |cols| {
                self.render_part_editor(&mut cols[0], s.common.cho, HangulComponent::Cho, cho_jamo, cho_gid.clone());
                self.render_part_editor(&mut cols[1], s.common.jung, HangulComponent::Jung, jung_jamo, jung_gid.clone());
                self.render_part_editor(&mut cols[2], s.common.jong, HangulComponent::Jong, jong_jamo, jong_gid.clone());
            });
        } else {
            ui.horizontal(|ui| {
                let jong_label = if jong_idx != 0 {
                    format!("{} ({})", s.common.jong, jong_jamo)
                } else {
                    s.common.jong.to_string()
                };
                if ui
                    .selectable_label(self.drawing.selected_part_tab == 0, format!("{} ({})", s.common.cho, cho_jamo))
                    .clicked()
                {
                    self.drawing.selected_part_tab = 0;
                }
                if ui
                    .selectable_label(self.drawing.selected_part_tab == 1, format!("{} ({})", s.common.jung, jung_jamo))
                    .clicked()
                {
                    self.drawing.selected_part_tab = 1;
                }
                if ui.selectable_label(self.drawing.selected_part_tab == 2, jong_label).clicked() {
                    self.drawing.selected_part_tab = 2;
                }
            });
            ui.separator();

            match self.drawing.selected_part_tab {
                0 => self.render_part_editor(ui, s.common.cho, HangulComponent::Cho, cho_jamo, cho_gid),
                1 => self.render_part_editor(ui, s.common.jung, HangulComponent::Jung, jung_jamo, jung_gid),
                _ => self.render_part_editor(ui, s.common.jong, HangulComponent::Jong, jong_jamo, jong_gid),
            }
        }
    }

    pub fn render_preview_canvas(&mut self, ui: &mut egui::Ui, zoom: f32, layout: Option<&LayoutResult>) {
        let canvas_w = self.project.canvas_w as f32 * zoom;
        let canvas_h = self.project.canvas_h as f32 * zoom;

        let (response, painter) = ui.allocate_painter(egui::vec2(canvas_w, canvas_h), egui::Sense::hover());
        let rect = response.rect;

        let dark = ui.visuals().dark_mode;
        painter.rect_filled(
            rect,
            0.0,
            if dark {
                egui::Color32::BLACK
            } else {
                egui::Color32::from_gray(245)
            },
        );
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
            egui::StrokeKind::Middle,
        );

        let grid_stroke = egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_gray(25)
            } else {
                egui::Color32::from_gray(200)
            },
        );
        for x in 0..=self.project.canvas_w {
            let fx = rect.min.x + x as f32 * zoom;
            painter.line_segment([egui::pos2(fx, rect.min.y), egui::pos2(fx, rect.max.y)], grid_stroke);
        }
        for y in 0..=self.project.canvas_h {
            let fy = rect.min.y + y as f32 * zoom;
            painter.line_segment([egui::pos2(rect.min.x, fy), egui::pos2(rect.max.x, fy)], grid_stroke);
        }

        let Some(res) = layout else {
            return;
        };

        let cur_key = (self.selected_char, self.project.canvas_w, self.project.canvas_h);
        if self.drawing.preview_dirty || self.drawing.preview_key != cur_key {
            self.drawing.preview_pixels = compose_pixels(&self.project.store, res, self.project.canvas_w, self.project.canvas_h);
            self.drawing.preview_key = cur_key;
            self.drawing.preview_dirty = false;
        }

        let ink = if dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
        for px in &self.drawing.preview_pixels {
            let pos = rect.min + egui::vec2(px.x as f32 * zoom, px.y as f32 * zoom);
            painter.rect_filled(egui::Rect::from_min_size(pos, egui::vec2(zoom, zoom)), 0.0, ink);
        }
    }

    fn render_part_editor(&mut self, ui: &mut egui::Ui, title: &str, kind: HangulComponent, jamo: char, group_id: Option<String>) {
        let s = crate::i18n::t(self.lang);
        let has_clipboard = self.drawing.pixel_clipboard.is_some();
        let mut copy_clicked = false;
        let mut paste_clicked = false;

        let inner = ui.group(|ui| -> Option<(hangul_syllable::GlyphKey, egui::Response)> {
            let header_top = ui.cursor().min;

            ui.horizontal(|ui| {
                ui.strong(title);
                ui.add_space(6.0);
                ui.label(jamo.to_string());
            });

            let Some(gid) = group_id else {
                ui.weak(s.common.none);
                return None;
            };

            let group_name = group_display_name(&self.project.engine.rules.groups, &gid);
            ui.small(format!("group: {}", group_name));
            let header_bottom_y = ui.cursor().min.y;

            self.project.store.ensure_glyph(hangul_syllable::GlyphKey::new(kind, jamo, &gid));
            let key = hangul_syllable::GlyphKey::new(kind, jamo, gid);
            self.render_local_pixel_editor(ui, key.clone());

            let header_rect = egui::Rect::from_min_max(header_top, egui::pos2(ui.max_rect().max.x, header_bottom_y));
            let header_resp = ui.allocate_rect(header_rect, egui::Sense::click());
            Some((key, header_resp))
        });

        if let Some((k, header_resp)) = &inner.inner {
            header_resp.context_menu(|ui| {
                if ui.button(s.common.copy).clicked() {
                    copy_clicked = true;
                    ui.close();
                }
                ui.add_enabled_ui(has_clipboard, |ui| {
                    if ui.button(s.common.paste).clicked() {
                        paste_clicked = true;
                        ui.close();
                    }
                });
            });
            if copy_clicked {
                self.copy_pixels_from(k);
            }
            if paste_clicked {
                self.paste_pixels_to(k);
            }
        }
    }

    fn render_preview_and_info(&mut self, ui: &mut egui::Ui, layout: Option<&LayoutResult>) {
        let s = crate::i18n::t(self.lang);
        let char_display = self.selected_char.to_string();
        let unicode_str = format!("U+{:04X}", self.selected_char as u32);
        let jamo_str = decompose_hangul(self.selected_char).map(|(cho, jung, jong)| {
            let cho_ch = get_jamo_char(HangulComponent::Cho, cho);
            let jung_ch = get_jamo_char(HangulComponent::Jung, jung);
            if jong != 0 {
                let jong_ch = get_jamo_char(HangulComponent::Jong, jong);
                format!("{cho_ch} + {jung_ch} + {jong_ch}")
            } else {
                format!("{cho_ch} + {jung_ch}")
            }
        });

        let layout_info = layout.map(|res| {
            let cho_name = group_display_name(&self.project.engine.rules.groups, &res.cho.group_id);
            let jung_name = group_display_name(&self.project.engine.rules.groups, &res.jung.group_id);
            let jong_name = res
                .jong
                .as_ref()
                .map(|j| group_display_name(&self.project.engine.rules.groups, &j.group_id));
            (
                res.selector_name.clone(),
                res.template_name.clone(),
                res.matched_variants.clone(),
                cho_name,
                jung_name,
                jong_name,
                res.fallbacks.iter().map(|f| f.message.clone()).collect::<Vec<_>>(),
            )
        });

        let preview_zoom = (200.0_f32 / self.project.canvas_w.max(self.project.canvas_h) as f32).clamp(1.0, 12.0);

        ui.horizontal(|ui| {
            self.render_preview_canvas(ui, preview_zoom, layout);

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&char_display)
                            .size(28.0)
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(&unicode_str).weak());
                    if let Some(jamo) = &jamo_str {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(jamo.as_str()).weak());
                    }
                });

                ui.add_space(6.0);

                match &layout_info {
                    Some((selector_name, template_name, matched_variants, cho_name, jung_name, jong_name, warnings)) => {
                        egui::ScrollArea::horizontal()
                            .id_salt("pipeline_scroll")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                ui.horizontal(|ui| {
                                    pipeline_stage(ui, s.drawing.selector, selector_name);
                                    pipeline_arrow(ui);
                                    pipeline_stage(ui, s.common.template_label, template_name);
                                    if !matched_variants.is_empty() {
                                        pipeline_arrow(ui);
                                        let variant_text = matched_variants.join(", ");
                                        pipeline_stage(ui, s.drawing.variant, &variant_text);
                                    }
                                    pipeline_arrow(ui);
                                    pipeline_stage(ui, s.common.cho, cho_name);
                                    pipeline_stage(ui, s.common.jung, jung_name);
                                    pipeline_stage(ui, s.common.jong, jong_name.as_deref().unwrap_or(s.drawing.no_jong));

                                    if !warnings.is_empty() {
                                        ui.add_space(6.0);
                                        ui.label(egui::RichText::new(format!("⚠ {}", warnings.len())).color(ui.visuals().warn_fg_color))
                                            .on_hover_ui(|ui| {
                                                for w in warnings {
                                                    ui.label(format!("- {w}"));
                                                }
                                            });
                                    }
                                });
                            });
                    }
                    None => {
                        ui.colored_label(ui.visuals().error_fg_color, s.drawing.layout_failed);
                    }
                }
            });
        });
    }
}

fn pipeline_stage(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.bg_stroke.color)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(7))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).small().weak());
                ui.label(egui::RichText::new(value));
            });
        });
}

fn pipeline_arrow(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new("▶").weak());
    ui.add_space(2.0);
}
