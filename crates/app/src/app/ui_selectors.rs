use eframe::egui;

use crate::app::{
    editor::FontEditor,
    ui_widgets::{move_vec, show_syllable_grid_two_tone, ui_charset_toggle, ui_separator_soft, visible_width},
};
use hangul_syllable::core::hangul::{
    NO_JONG, cho_allowed, cho_allowed_ext, decompose_hangul, get_jamo_char, jong_allowed_with_none, jong_allowed_with_none_ext,
    jung_allowed, jung_allowed_ext,
};
use hangul_syllable::{CharSetCond, HangulComponent, SelectorRule};

impl FontEditor {
    pub fn ui_selector_rules_mode(&mut self, ui: &mut egui::Ui) {
        use crate::app::ui_widgets::sub_panel_tabs;

        if ui.available_width() < crate::app::editor::NARROW_WIDTH {
            let s = crate::i18n::t(self.lang);
            sub_panel_tabs(ui, &mut self.narrow_sel_sub, &[s.common.list, s.common.edit, s.common.preview]);
            ui.separator();
            match self.narrow_sel_sub {
                0 => self.render_sel_list_content(ui),
                1 => {
                    let content_width = visible_width(ui);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_width(content_width)
                        .show(ui, |ui| {
                            ui.set_width(content_width);
                            ui.set_max_width(content_width);
                            if self.render_sel_editor_content(ui) {
                                self.invalidate_render_caches();
                            }
                        });
                }
                2 => self.render_sel_preview_content(ui),
                _ => {}
            }
            return;
        }

        egui::Panel::left("sel_list").min_size(280.0).max_size(500.0).show_inside(ui, |ui| {
            self.render_sel_list_content(ui);
        });

        egui::Panel::right("sel_preview")
            .min_size(320.0)
            .max_size(600.0)
            .default_size(340.0)
            .show_inside(ui, |ui| {
                self.render_sel_preview_content(ui);
            });

        let changed = {
            let mut changed = false;
            egui::CentralPanel::default().show_inside(ui, |ui| {
                changed = self.render_sel_editor_content(ui);
            });
            changed
        };
        if changed {
            self.invalidate_render_caches();
        }
    }

    fn render_sel_list_content(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        #[derive(Clone, Debug)]
        enum Cmd {
            Add,
            DeleteSelected,
            DuplicateSelected,
            SelectById(String),
            MoveUp { idx: usize },
            MoveDown { idx: usize },
            MoveTop { idx: usize },
            MoveBottom { idx: usize },
            SortByName,
            DuplicateAt { idx: usize },
            DeleteAt { idx: usize },
        }

        let mut cmd: Option<Cmd> = None;

        ui.horizontal_wrapped(|ui| {
            if ui.button(s.common.add).clicked() {
                cmd = Some(Cmd::Add);
            }
            ui.add_enabled_ui(!self.project.engine.rules.selectors.is_empty(), |ui| {
                if ui.button(s.common.copy).clicked() {
                    cmd = Some(Cmd::DuplicateSelected);
                }
                if ui.button(s.common.delete).clicked() {
                    cmd = Some(Cmd::DeleteSelected);
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();

        let selectors_snapshot: Vec<(String, String, i32, String)> = self
            .project
            .engine
            .rules
            .selectors
            .iter()
            .map(|r| {
                let tpl_name = self
                    .project
                    .engine
                    .rules
                    .templates
                    .iter()
                    .find(|t| t.id == r.template_id)
                    .map(|t| if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() }.to_string())
                    .unwrap_or_else(|| r.template_id.clone());
                (r.id.clone(), r.name.clone(), r.priority, tpl_name)
            })
            .collect();

        let selected_id_before = self
            .project
            .engine
            .rules
            .selectors
            .get(self.selected_selector)
            .map(|r| r.id.clone());

        ui.add_space(4.0);
        ui.label(egui::RichText::new(s.selectors.rule_list).weak());

        egui::ScrollArea::vertical()
            .id_salt("selectors_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;

                    for (i, (rid, rname, prio, tpl_id)) in selectors_snapshot.iter().enumerate() {
                        let selected = i == self.selected_selector;
                        let display = if rname.is_empty() { rid.as_str() } else { rname.as_str() };
                        let text = format!("[{}] {} -> {}", prio, display, tpl_id);

                        let resp = ui.add_sized(
                            egui::vec2(ui.available_width(), 24.0),
                            egui::Button::new(text).selected(selected).wrap(),
                        );

                        if resp.clicked() {
                            cmd = Some(Cmd::SelectById(rid.clone()));
                        }

                        resp.context_menu(|ui| {
                            if ui.button(s.common.move_up).clicked() {
                                cmd = Some(Cmd::MoveUp { idx: i });
                                ui.close();
                            }
                            if ui.button(s.common.move_down).clicked() {
                                cmd = Some(Cmd::MoveDown { idx: i });
                                ui.close();
                            }
                            if ui.button(s.common.move_top).clicked() {
                                cmd = Some(Cmd::MoveTop { idx: i });
                                ui.close();
                            }
                            if ui.button(s.common.move_bottom).clicked() {
                                cmd = Some(Cmd::MoveBottom { idx: i });
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(s.common.sort_by_name).clicked() {
                                cmd = Some(Cmd::SortByName);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(s.common.copy).clicked() {
                                cmd = Some(Cmd::DuplicateAt { idx: i });
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(s.common.delete).clicked() {
                                cmd = Some(Cmd::DeleteAt { idx: i });
                                ui.close();
                            }
                        });
                    }
                });
            });

        ui.separator();
        ui.label(s.selectors.priority_note);

        if let Some(cmd) = cmd {
            let restore_selected = |editor: &mut FontEditor, sel_id: Option<String>| {
                if let Some(id) = sel_id {
                    if let Some(new_idx) = editor.project.engine.rules.selectors.iter().position(|r| r.id == id) {
                        editor.selected_selector = new_idx;
                    } else {
                        editor.selected_selector = 0;
                    }
                } else {
                    editor.selected_selector = 0;
                }
            };

            let mut cmd_mutated = false;
            match cmd {
                Cmd::Add => {
                    let tpl_id = self
                        .project
                        .engine
                        .rules
                        .templates
                        .first()
                        .map(|t| t.id.clone())
                        .unwrap_or_else(|| "None".into());

                    let new_id = self.project.engine.rules.new_selector_id();
                    self.project.engine.rules.add_selector(SelectorRule {
                        id: new_id,
                        name: String::new(),
                        priority: 100,
                        cho_set: CharSetCond::Any,
                        jung_set: CharSetCond::Any,
                        jong_set: CharSetCond::Any,
                        template_id: tpl_id,
                    });
                    self.selected_selector = self.project.engine.rules.selectors.len().saturating_sub(1);
                    cmd_mutated = true;
                }

                Cmd::DeleteSelected => {
                    if !self.project.engine.rules.selectors.is_empty() {
                        let idx = self.selected_selector.min(self.project.engine.rules.selectors.len() - 1);
                        self.project.engine.rules.selectors.remove(idx);
                        self.selected_selector = self.selected_selector.saturating_sub(1);
                        cmd_mutated = true;
                    }
                }

                Cmd::DuplicateSelected => {
                    if !self.project.engine.rules.selectors.is_empty() {
                        let idx = self.selected_selector.min(self.project.engine.rules.selectors.len() - 1);
                        let src = &self.project.engine.rules.selectors[idx];
                        let base_name = if src.name.is_empty() { src.id.clone() } else { src.name.clone() };
                        let mut cloned = src.clone();
                        cloned.id = self.project.engine.rules.new_selector_id();
                        cloned.name = format!("{}{}", base_name, s.common.copy_suffix);
                        self.project.engine.rules.selectors.insert(idx + 1, cloned);
                        self.selected_selector = (idx + 1).min(self.project.engine.rules.selectors.len().saturating_sub(1));
                        cmd_mutated = true;
                    }
                }

                Cmd::SelectById(id) => {
                    if let Some(idx) = self.project.engine.rules.selectors.iter().position(|r| r.id == id) {
                        self.selected_selector = idx;
                    }
                }

                Cmd::MoveUp { idx } => {
                    if idx > 0 {
                        let sel_id = selected_id_before;
                        self.project.engine.rules.selectors.swap(idx, idx - 1);
                        restore_selected(self, sel_id);
                        cmd_mutated = true;
                    }
                }

                Cmd::MoveDown { idx } => {
                    if idx + 1 < self.project.engine.rules.selectors.len() {
                        let sel_id = selected_id_before;
                        self.project.engine.rules.selectors.swap(idx, idx + 1);
                        restore_selected(self, sel_id);
                        cmd_mutated = true;
                    }
                }

                Cmd::MoveTop { idx } => {
                    let sel_id = selected_id_before;
                    move_vec(&mut self.project.engine.rules.selectors, idx, 0);
                    restore_selected(self, sel_id);
                    cmd_mutated = true;
                }

                Cmd::MoveBottom { idx } => {
                    let sel_id = selected_id_before;
                    let n = self.project.engine.rules.selectors.len();
                    move_vec(&mut self.project.engine.rules.selectors, idx, n.saturating_sub(1));
                    restore_selected(self, sel_id);
                    cmd_mutated = true;
                }

                Cmd::SortByName => {
                    let sel_id = selected_id_before;
                    self.project.engine.rules.selectors.sort_by(|a, b| {
                        let na = if a.name.is_empty() { &a.id } else { &a.name };
                        let nb = if b.name.is_empty() { &b.id } else { &b.name };
                        na.cmp(nb)
                    });
                    restore_selected(self, sel_id);
                    cmd_mutated = true;
                }

                Cmd::DuplicateAt { idx } => {
                    if idx < self.project.engine.rules.selectors.len() {
                        let src = &self.project.engine.rules.selectors[idx];
                        let base_name = if src.name.is_empty() { src.id.clone() } else { src.name.clone() };
                        let mut cloned = src.clone();
                        cloned.id = self.project.engine.rules.new_selector_id();
                        cloned.name = format!("{}{}", base_name, s.common.copy_suffix);
                        self.project.engine.rules.selectors.insert(idx + 1, cloned);
                        self.selected_selector = (idx + 1).min(self.project.engine.rules.selectors.len().saturating_sub(1));
                        cmd_mutated = true;
                    }
                }

                Cmd::DeleteAt { idx } => {
                    if idx < self.project.engine.rules.selectors.len() {
                        self.project.engine.rules.selectors.remove(idx);
                        self.selected_selector = self
                            .selected_selector
                            .min(self.project.engine.rules.selectors.len().saturating_sub(1));
                        cmd_mutated = true;
                    }
                }
            }
            if cmd_mutated {
                self.invalidate_render_caches();
            }
        }
    }

    fn render_sel_preview_content(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);
        let sel_id = self
            .project
            .engine
            .rules
            .selectors
            .get(self.selected_selector)
            .map(|r| r.id.clone())
            .unwrap_or_default();

        if sel_id.is_empty() {
            ui.weak(s.selectors.no_selector);
            return;
        }

        if self.sel_editor.cache_key.as_deref() != Some(&sel_id) {
            let (eff, ov) = selector_char_split(self, &sel_id);
            self.sel_editor.effective_chars = eff;
            self.sel_editor.overridden_chars = ov;
            self.sel_editor.cache_key = Some(sel_id.clone());
        }

        let eff_n = self.sel_editor.effective_chars.len();
        let ov_n = self.sel_editor.overridden_chars.len();
        if ov_n > 0 {
            ui.label(format!(
                "{}{}  ({}{} {})",
                eff_n, s.common.char_suffix, ov_n, s.common.char_suffix, s.selectors.overridden_note,
            ));
        } else {
            ui.label(format!("{}{}", eff_n, s.common.char_suffix));
        }
        ui.add_space(4.0);

        show_syllable_grid_two_tone(
            ui,
            "sel_preview_grid",
            &self.sel_editor.effective_chars.clone(),
            &self.sel_editor.overridden_chars.clone(),
            &self.project.engine,
            &self.project.store,
            self.project.canvas_w,
            self.project.canvas_h,
            s.common.no_syllables,
        );
    }

    fn render_sel_editor_content(&mut self, ui: &mut egui::Ui) -> bool {
        ui.set_max_width(visible_width(ui));
        let s = crate::i18n::t(self.lang);

        if self.project.engine.rules.selectors.is_empty() {
            ui.label(s.selectors.no_rules);
            return false;
        }

        let mut r = self.project.engine.rules.selectors[self.selected_selector].clone();
        let mut changed = false;

        let mut widget_ch = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{}:", s.common.name));
            if ui
                .add(egui::TextEdit::singleline(&mut r.name).desired_width(ui.available_width().clamp(120.0, 320.0)))
                .changed()
            {
                widget_ch = true;
            }
            ui.label(s.selectors.priority);
            if ui.add(egui::DragValue::new(&mut r.priority).range(-10000..=10000)).changed() {
                widget_ch = true;
            }
        });
        if widget_ch {
            changed = true;
        }
        ui_separator_soft(ui);

        let old_cho_set = r.cho_set.clone();
        let old_jung_set = r.jung_set.clone();
        let old_jong_set = r.jong_set.clone();
        let cho_ext: Vec<char>;
        let cho_set: &[char] = if self.project.old_hangul_enabled {
            cho_ext = cho_allowed_ext();
            &cho_ext
        } else {
            cho_allowed()
        };
        let jung_ext: Vec<char>;
        let jung_set: &[char] = if self.project.old_hangul_enabled {
            jung_ext = jung_allowed_ext();
            &jung_ext
        } else {
            jung_allowed()
        };
        let jong_ext: Vec<char>;
        let jong_set: &[char] = if self.project.old_hangul_enabled {
            jong_ext = jong_allowed_with_none_ext();
            &jong_ext
        } else {
            jong_allowed_with_none()
        };
        ui_charset_toggle(
            ui,
            ui.make_persistent_id("sel_cho_set"),
            s.selectors.cho_filter,
            &mut r.cho_set,
            cho_set,
            10,
            self.lang,
        );
        ui_charset_toggle(
            ui,
            ui.make_persistent_id("sel_jung_set"),
            s.selectors.jung_filter,
            &mut r.jung_set,
            jung_set,
            10,
            self.lang,
        );
        ui_charset_toggle(
            ui,
            ui.make_persistent_id("sel_jong_set"),
            s.selectors.jong_filter,
            &mut r.jong_set,
            jong_set,
            10,
            self.lang,
        );
        if r.cho_set != old_cho_set || r.jung_set != old_jung_set || r.jong_set != old_jong_set {
            changed = true;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.label(s.selectors.result_template);

        let tpl_options: Vec<(String, String)> = self
            .project
            .engine
            .rules
            .templates
            .iter()
            .map(|t| (t.id.clone(), if t.name.is_empty() { t.id.clone() } else { t.name.clone() }))
            .collect();
        if tpl_options.is_empty() {
            ui.label(s.selectors.no_templates);
        } else {
            let old_tpl_id = r.template_id.clone();
            let selected_text = tpl_options
                .iter()
                .find(|(id, _)| *id == r.template_id)
                .map(|(_, name)| name.as_str())
                .unwrap_or(r.template_id.as_str())
                .to_string();
            egui::ComboBox::from_id_salt(ui.make_persistent_id("sel_tpl_pick"))
                .selected_text(selected_text)
                .width(ui.available_width().clamp(120.0, 320.0))
                .show_ui(ui, |ui| {
                    for (tid, tname) in &tpl_options {
                        ui.selectable_value(&mut r.template_id, tid.clone(), tname.as_str());
                    }
                });
            if r.template_id != old_tpl_id {
                changed = true;
            }
        }

        self.project.engine.rules.selectors[self.selected_selector] = r;
        changed
    }
}

fn selector_char_split(editor: &FontEditor, sel_id: &str) -> (Vec<char>, Vec<char>) {
    let Some(sel) = editor.project.engine.rules.find_selector(sel_id) else {
        return (vec![], vec![]);
    };
    let sel = sel.clone();

    let mut sorted_sels: Vec<&SelectorRule> = editor.project.engine.rules.selectors.iter().collect();
    sorted_sels.sort_unstable_by_key(|s| std::cmp::Reverse(s.priority));

    let mut effective = Vec::new();
    let mut overridden = Vec::new();

    for code in 0xAC00u32..=0xD7A3u32 {
        let Some(ch) = char::from_u32(code) else { continue };
        let Some((cho_idx, jung_idx, jong_idx)) = decompose_hangul(ch) else {
            continue;
        };
        let cho_ch = get_jamo_char(HangulComponent::Cho, cho_idx);
        let jung_ch = get_jamo_char(HangulComponent::Jung, jung_idx);
        let jong_ch = (jong_idx != 0).then(|| get_jamo_char(HangulComponent::Jong, jong_idx));
        let jong_for_match = jong_ch.unwrap_or(NO_JONG);

        if !sel.cho_set.matches(cho_ch) || !sel.jung_set.matches(jung_ch) || !sel.jong_set.matches(jong_for_match) {
            continue;
        }

        let winning_id = sorted_sels
            .iter()
            .find(|s| s.cho_set.matches(cho_ch) && s.jung_set.matches(jung_ch) && s.jong_set.matches(jong_for_match))
            .map(|s| s.id.as_str());

        if winning_id == Some(sel_id) {
            effective.push(ch);
        } else {
            overridden.push(ch);
        }
    }

    (effective, overridden)
}
