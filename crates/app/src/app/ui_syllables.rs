use eframe::egui;

use crate::app::{
    editor::{EditorMode, FontEditor},
    ui_hangul_browser::{combo_group_filter, combo_jong_filter, parse_hangul_query_to_char},
    ui_widgets::HangulCell,
};
use hangul_syllable::core::{
    hangul::{decompose_hangul, get_jamo_char},
    render::compose_pixels,
};

impl FontEditor {
    pub fn ui_syllables_mode(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);
        ui.add_space(6.0);

        let mut trigger_jump = false;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.hangul_browser.search)
                        .hint_text(s.syllables.search_hint)
                        .desired_width(120.0),
                );

                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    trigger_jump = true;
                }
                if ui.button(s.syllables.go).clicked() {
                    trigger_jump = true;
                }

                ui.separator();

                egui::Grid::new("syllable_filters_top")
                    .num_columns(8)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(s.common.template_label);
                        let cur_tpl = self
                            .hangul_browser
                            .template_id
                            .as_deref()
                            .and_then(|id| self.project.engine.rules.templates.iter().find(|t| t.id == id))
                            .map(|t| if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() })
                            .unwrap_or(s.widgets.any)
                            .to_string();
                        egui::ComboBox::from_id_salt("top_filter_tpl")
                            .selected_text(cur_tpl)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(self.hangul_browser.template_id.is_none(), s.widgets.any)
                                    .clicked()
                                {
                                    self.hangul_browser.template_id = None;
                                }
                                for t in &self.project.engine.rules.templates {
                                    let display = if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() };
                                    let is_sel = self.hangul_browser.template_id.as_deref() == Some(&t.id);
                                    if ui.selectable_label(is_sel, display).clicked() {
                                        self.hangul_browser.template_id = Some(t.id.clone());
                                    }
                                }
                            });

                        ui.label(format!("{}:", s.common.cho));
                        let mut cho_gid = self.hangul_browser.cho_group_id.clone();
                        combo_group_filter(
                            ui,
                            hangul_syllable::core::types::HangulComponent::Cho,
                            &self.project.engine.rules.groups,
                            &mut cho_gid,
                            "top_f_cho",
                            self.lang,
                        );
                        self.hangul_browser.cho_group_id = cho_gid;

                        ui.label(format!("{}:", s.common.jung));
                        let mut jung_gid = self.hangul_browser.jung_group_id.clone();
                        combo_group_filter(
                            ui,
                            hangul_syllable::core::types::HangulComponent::Jung,
                            &self.project.engine.rules.groups,
                            &mut jung_gid,
                            "top_f_jung",
                            self.lang,
                        );
                        self.hangul_browser.jung_group_id = jung_gid;

                        ui.label(format!("{}:", s.common.jong));
                        let mut jong_filter = self.hangul_browser.jong_group_id.clone();
                        combo_jong_filter(ui, &self.project.engine.rules.groups, &mut jong_filter, "top_f_jong", self.lang);
                        self.hangul_browser.jong_group_id = jong_filter;
                    });

                ui.separator();
                ui.add(egui::Slider::new(&mut self.hangul_browser.cell_px, 32.0..=128.0).text(s.syllables.cell_size));
            });
        });

        ui.add_space(6.0);

        let list = self.collect_filtered_hangul_list_by_rules();

        if trigger_jump && let Some(ch) = parse_hangul_query_to_char(&self.hangul_browser.search) {
            self.selected_char = ch;
            self.target_text = ch.to_string();

            if let Some(idx) = list.iter().position(|&x| x == ch) {
                self.hangul_browser.jump_request_idx = Some(idx);
            }
        }

        ui.label(format!("{}{}{}", s.syllables.results_prefix, list.len(), s.common.char_suffix));

        self.render_filtered_grid_full(ui, &list);
    }

    fn render_filtered_grid_full(&mut self, ui: &mut egui::Ui, list: &[char]) {
        let s = crate::i18n::t(self.lang);
        let spacing = 6.0;
        let cell = self.hangul_browser.cell_px;

        if list.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(s.syllables.no_results);
            });
            return;
        }

        ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);

        if self.hangul_browser.cache_em != (self.project.canvas_w, self.project.canvas_h) {
            self.hangul_browser.pixel_cache.clear();
            self.hangul_browser.cache_em = (self.project.canvas_w, self.project.canvas_h);
        }

        let available_w = ui.clip_rect().width().max(1.0);
        let cols = ((available_w + spacing) / (cell + spacing)).floor() as usize;
        let cols = cols.max(1);

        let total_count = list.len();
        let total_rows = total_count.div_ceil(cols);

        let row_h = cell;
        let row_advance = row_h + spacing;

        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("syllable_full_grid")
            .auto_shrink([false, false]);

        if let Some(idx) = self.hangul_browser.jump_request_idx.take() {
            self.hangul_browser.jump_highlight_idx = Some(idx);

            let target_row = idx / cols;
            let viewport_height = ui.available_height();
            let row_y = target_row as f32 * row_advance;
            let center_offset = row_y - (viewport_height * 0.5) + (row_h * 0.5);
            scroll = scroll.vertical_scroll_offset(center_offset.max(0.0));
            ui.ctx().request_repaint();
        }

        scroll.show_rows(ui, row_h, total_rows, |ui, row_range| {
            for row in row_range {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if idx >= total_count {
                            break;
                        }
                        let ch = list[idx];

                        if !self.hangul_browser.pixel_cache.contains_key(&ch) {
                            let pixels = self
                                .project
                                .engine
                                .layout_char(&self.project.store, ch, decompose_hangul, get_jamo_char)
                                .map(|res| compose_pixels(&self.project.store, &res, self.project.canvas_w, self.project.canvas_h))
                                .unwrap_or_default();
                            self.hangul_browser.pixel_cache.insert(ch, pixels);
                        }

                        let pixels = self.hangul_browser.pixel_cache.get(&ch).map(|v| v.as_slice());
                        let is_selected = self.selected_char == ch;
                        let resp = ui.add(HangulCell::new(
                            ch,
                            pixels,
                            self.project.canvas_w,
                            self.project.canvas_h,
                            cell,
                            is_selected,
                        ));

                        if resp.clicked() {
                            self.selected_char = ch;
                            self.target_text = ch.to_string();
                            self.mode = EditorMode::Drawing;
                        }
                    }
                });
            }
        });
    }
}
