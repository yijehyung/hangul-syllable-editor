use eframe::egui::{self, global_theme_preference_switch};
#[cfg(not(target_arch = "wasm32"))]
use hangul_syllable::{load_project_from_path, save_project_to_path};

#[cfg(target_arch = "wasm32")]
use crate::app::deferred::Deferred;
use crate::app::editor::{DiscardAction, EditorMode, FontEditor};

impl FontEditor {
    pub fn render_top_panel(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        ui.horizontal(|ui| {
            ui.menu_button(s.top.file, |ui| {
                ui.menu_button(s.top.new_menu, |ui| {
                    let presets: &[(_, DiscardAction)] = &[
                        (s.top.preset_default, DiscardAction::NewDefault),
                        (s.top.preset_minzkn, DiscardAction::NewMinzkn),
                        (s.top.preset_zik, DiscardAction::NewZik),
                        (s.top.preset_dkb, DiscardAction::NewDkb),
                        (s.top.preset_hanterm, DiscardAction::NewHanterm),
                    ];
                    for (label, action) in presets {
                        if ui.button(*label).clicked() {
                            if self.project.is_dirty {
                                self.pending_discard = Some(action.clone());
                                self.show_discard_dialog = true;
                            } else {
                                self.execute_discard_action(action.clone(), ui.ctx().clone());
                            }
                            ui.close();
                        }
                    }
                });

                ui.separator();

                let opening = self.open_promise.is_some();
                if ui.add_enabled(!opening, egui::Button::new(s.top.open)).clicked() {
                    ui.close();
                    if self.project.is_dirty {
                        self.pending_discard = Some(DiscardAction::OpenFile);
                        self.show_discard_dialog = true;
                    } else {
                        self.do_open_project(ui.ctx().clone());
                    }
                }

                if ui.add_enabled(self.project.is_dirty, egui::Button::new(s.top.save)).clicked() {
                    ui.close();
                    self.save_project_now();
                }

                if ui.button(s.top.save_as).clicked() {
                    ui.close();
                    self.save_project_as();
                }

                ui.separator();

                if ui.button(s.top.export_image).clicked() {
                    ui.close();
                    self.open_export_window();
                }
            });

            ui.separator();
            ui.label(s.top.canvas);
            let changed_w = ui
                .add(egui::DragValue::new(&mut self.project.canvas_w).suffix("px").range(8..=64))
                .changed();
            ui.label("×");
            let changed_h = ui
                .add(egui::DragValue::new(&mut self.project.canvas_h).suffix("px").range(8..=64))
                .changed();
            if changed_w || changed_h {
                self.project.is_dirty = true;
                self.invalidate_render_caches();
            }

            ui.separator();
            ui.checkbox(&mut self.drawing.pen_toggle_mode, s.drawing.pen_toggle);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("🌐", |ui| {
                    if ui.selectable_label(self.lang == crate::i18n::Lang::Ko, "한국어").clicked() {
                        self.lang = crate::i18n::Lang::Ko;
                        ui.close();
                    }
                    if ui.selectable_label(self.lang == crate::i18n::Lang::En, "English").clicked() {
                        self.lang = crate::i18n::Lang::En;
                        ui.close();
                    }
                });

                global_theme_preference_switch(ui);
            });
        });

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, EditorMode::Drawing, s.top.tab_drawing);
            ui.selectable_value(&mut self.mode, EditorMode::Components, s.top.tab_components);
            ui.selectable_value(&mut self.mode, EditorMode::Templates, s.top.tab_templates);
            ui.selectable_value(&mut self.mode, EditorMode::SelectorRules, s.top.tab_selectors);
            ui.selectable_value(&mut self.mode, EditorMode::Syllables, s.top.tab_syllables);
            ui.selectable_value(&mut self.mode, EditorMode::TypeTest, s.top.tab_type_test);
            if self.project.old_hangul_enabled {
                ui.selectable_value(&mut self.mode, EditorMode::OldHangulMap, s.top.tab_old_hangul_map);
            }
        });
    }

    pub(super) fn do_open_project(&mut self, _ctx: egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new().add_filter("Hangul Font Project", &["hangul"]).pick_file() {
                match load_project_from_path(&path) {
                    Ok(data) => {
                        self.project.current_path = Some(path);
                        self.apply_project(data);
                    }
                    Err(e) => self.push_error(format!("프로젝트 열기 실패: {e}")),
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.open_promise = Some(Deferred::spawn_async(
                async {
                    let h = rfd::AsyncFileDialog::new()
                        .add_filter("Hangul Font Project", &["hangul"])
                        .pick_file()
                        .await?;
                    let name = h.file_name();
                    let bytes = h.read().await;
                    Some((bytes, Some(std::path::PathBuf::from(name))))
                },
                _ctx,
            ));
        }
    }

    pub fn save_project_now(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut data = self.project.to_project_data();
            if let Some(path) = self.project.current_path.clone() {
                match save_project_to_path(&path, &mut data) {
                    Ok(()) => self.project.is_dirty = false,
                    Err(e) => self.push_error(format!("저장 실패: {e}")),
                }
                return;
            }
        }
        self.save_project_as();
    }

    fn save_project_as(&mut self) {
        let mut data = self.project.to_project_data();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Hangul Font Project", &["hangul"])
                .set_file_name("untitled.hangul")
                .save_file()
            {
                match save_project_to_path(&path, &mut data) {
                    Ok(()) => {
                        self.project.current_path = Some(path);
                        self.project.is_dirty = false;
                    }
                    Err(e) => self.push_error(format!("저장 실패: {e}")),
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        if let Ok(yaml) = hangul_syllable::io::project::serialize_project_to_yaml(&mut data) {
            crate::platform::trigger_download(yaml.as_bytes(), "untitled.hangul");
            self.project.is_dirty = false;
        }
    }
}
