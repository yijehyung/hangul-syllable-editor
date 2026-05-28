use eframe::egui;
use hangul_syllable::core::hangul::{cho_allowed, jong_allowed, jung_allowed};
use hangul_syllable::{HangulComponent, default_archaic_map, jamo_matches_kind};

use crate::app::editor::FontEditor;

impl FontEditor {
    pub fn ui_old_hangul_mode(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.old_hangul_filter, None, s.old_hangul.filter_all);
            ui.selectable_value(&mut self.old_hangul_filter, Some(HangulComponent::Cho), s.common.cho);
            ui.selectable_value(&mut self.old_hangul_filter, Some(HangulComponent::Jung), s.common.jung);
            ui.selectable_value(&mut self.old_hangul_filter, Some(HangulComponent::Jong), s.common.jong);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(s.old_hangul.reset).clicked() {
                    self.project.archaic_jamo_map = default_archaic_map();
                    self.project.is_dirty = true;
                }
            });
        });

        ui.separator();

        let filter = self.old_hangul_filter;
        let cho_opts: Vec<char> = cho_allowed().to_vec();
        let jung_opts: Vec<char> = jung_allowed().to_vec();
        let jong_opts: Vec<char> = jong_allowed().to_vec();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            egui::Grid::new("archaic_map_grid")
                .num_columns(4)
                .striped(true)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.strong(s.old_hangul.col_archaic);
                    ui.strong("U+");
                    ui.label("");
                    ui.strong(s.old_hangul.col_modern);
                    ui.end_row();

                    for (i, (archaic, modern)) in self.project.archaic_jamo_map.iter_mut().enumerate() {
                        let kind = if jamo_matches_kind(*archaic, HangulComponent::Cho) {
                            HangulComponent::Cho
                        } else if jamo_matches_kind(*archaic, HangulComponent::Jung) {
                            HangulComponent::Jung
                        } else {
                            HangulComponent::Jong
                        };

                        if let Some(f) = filter
                            && f != kind
                        {
                            continue;
                        }

                        let opts: &[char] = match kind {
                            HangulComponent::Cho => &cho_opts,
                            HangulComponent::Jung => &jung_opts,
                            HangulComponent::Jong => &jong_opts,
                        };
                        let kind_label = match kind {
                            HangulComponent::Cho => s.common.cho,
                            HangulComponent::Jung => s.common.jung,
                            HangulComponent::Jong => s.common.jong,
                        };

                        ui.label(format!("{archaic}  {kind_label}"));
                        ui.label(format!("U+{:04X}", *archaic as u32));
                        ui.label("→");

                        egui::ComboBox::from_id_salt(("archaic_map", i))
                            .selected_text(format!("{modern}"))
                            .show_ui(ui, |ui| {
                                for &opt in opts {
                                    if ui.selectable_label(*modern == opt, format!("{opt}")).clicked() {
                                        *modern = opt;
                                        self.project.is_dirty = true;
                                    }
                                }
                            });

                        ui.end_row();
                    }
                });
        });
    }
}
