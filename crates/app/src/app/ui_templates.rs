use eframe::egui;

use crate::app::{
    editor::FontEditor,
    ui_widgets::{
        JamoTriplet, move_vec, pick_group_combo, show_jamo_triplet_grid, show_syllable_grid, ui_charset_toggle, ui_separator_soft,
    },
};
use hangul_syllable::core::hangul::{
    NO_JONG, cho_allowed, cho_allowed_ext, decompose_hangul, get_jamo_char, jong_allowed, jong_allowed_ext, jung_allowed, jung_allowed_ext,
};
use hangul_syllable::{CharSetCond, HangulComponent, Template, VariantRule};

impl FontEditor {
    pub fn ui_templates_mode(&mut self, ui: &mut egui::Ui) {
        use crate::app::ui_widgets::sub_panel_tabs;

        if ui.available_width() < crate::app::editor::NARROW_WIDTH {
            let s = crate::i18n::t(self.lang);
            sub_panel_tabs(ui, &mut self.narrow_tpl_sub, &[s.common.list, s.common.edit, s.common.preview]);
            ui.separator();
            match self.narrow_tpl_sub {
                0 => self.ui_template_list_panel(ui),
                1 => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if self.ui_template_settings(ui) {
                            self.invalidate_render_caches();
                        }
                    });
                }
                2 => self.ui_template_preview_panel(ui),
                _ => {}
            }
            return;
        }

        egui::Panel::left("tpl_left").min_size(200.0).max_size(400.0).show_inside(ui, |ui| {
            self.ui_template_list_panel(ui);
        });

        egui::Panel::right("tpl_preview")
            .min_size(320.0)
            .max_size(600.0)
            .default_size(340.0)
            .show_inside(ui, |ui| {
                self.ui_template_preview_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.ui_template_settings(ui) {
                    self.invalidate_render_caches();
                }
            });
        });
    }

    fn ui_template_list_panel(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        ui.separator();

        let templates_snapshot: Vec<(String, String)> = self
            .project
            .engine
            .rules
            .templates
            .iter()
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect();

        let selected_id_before = self
            .project
            .engine
            .rules
            .templates
            .get(self.tpl_editor.selected_template)
            .map(|t| t.id.clone());

        #[derive(Clone, Debug)]
        enum Cmd {
            SelectById(String),
            MoveTop { idx: usize },
            MoveBottom { idx: usize },
            MoveUp { idx: usize },
            MoveDown { idx: usize },
            SortByName,
            Copy { idx: usize },
            Delete { idx: usize },
        }

        let mut cmd: Option<Cmd> = None;

        let tpl_referenced = self
            .project
            .engine
            .rules
            .templates
            .get(self.tpl_editor.selected_template)
            .map(|t| self.project.engine.rules.is_template_referenced(&t.id))
            .unwrap_or(false);
        let can_delete_tpl = !self.project.engine.rules.templates.is_empty() && !tpl_referenced;

        ui.horizontal(|ui| {
            if ui.button(s.common.add).clicked() {
                let base_cho = self.project.engine.rules.base_cho_group_id.clone();
                let base_jung = self.project.engine.rules.base_jung_group_id.clone();
                let new_id = self.project.engine.rules.new_template_id();
                let tpl = Template {
                    id: new_id,
                    name: s.templates.new_template_name.into(),
                    default_cho_group_id: base_cho,
                    default_jung_group_id: base_jung,
                    default_jong_group_id: None,
                    variant_rules: vec![],
                };
                self.project.engine.rules.add_template(tpl);
                self.tpl_editor.selected_template = self.project.engine.rules.templates.len().saturating_sub(1);
                self.invalidate_render_caches();
            }

            if ui.button(s.common.copy).clicked()
                && let Some(t) = self.project.engine.rules.templates.get(self.tpl_editor.selected_template).cloned()
            {
                let mut t2 = t.clone();
                let base_name = if t.name.is_empty() { &t.id } else { &t.name };
                t2.id = self.project.engine.rules.new_template_id();
                t2.name = format!("{}{}", base_name, s.common.copy_suffix);
                self.project.engine.rules.templates.push(t2);
                self.tpl_editor.selected_template = self.project.engine.rules.templates.len().saturating_sub(1);
                self.invalidate_render_caches();
            }

            ui.add_enabled_ui(can_delete_tpl, |ui| {
                if ui.button(s.common.delete).clicked() {
                    let idx = self.tpl_editor.selected_template.min(self.project.engine.rules.templates.len() - 1);
                    self.project.engine.rules.templates.remove(idx);
                    self.tpl_editor.selected_template = self.tpl_editor.selected_template.saturating_sub(1);
                    self.invalidate_render_caches();
                }
            });
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("tpl_list_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, (tid, tname)) in templates_snapshot.iter().enumerate() {
                    let display = if tname.is_empty() { tid.as_str() } else { tname.as_str() };
                    let selected = i == self.tpl_editor.selected_template;
                    let resp = ui.add(egui::Button::new(display).selected(selected));

                    if resp.clicked() {
                        cmd = Some(Cmd::SelectById(tid.clone()));
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
                            cmd = Some(Cmd::Copy { idx: i });
                            ui.close();
                        }
                        ui.separator();
                        let referenced = self.project.engine.rules.is_template_referenced(tid);
                        ui.add_enabled_ui(!referenced, |ui| {
                            if ui.button(s.common.delete).clicked() {
                                cmd = Some(Cmd::Delete { idx: i });
                                ui.close();
                            }
                        });
                        if referenced {
                            ui.colored_label(ui.visuals().warn_fg_color, s.common.cannot_delete_referenced);
                        }
                    });
                }
            });

        if let Some(cmd) = cmd {
            let restore = |editor: &mut FontEditor, sel_id: Option<String>| {
                if let Some(id) = sel_id {
                    if let Some(new_idx) = editor.project.engine.rules.templates.iter().position(|t| t.id == id) {
                        editor.tpl_editor.selected_template = new_idx;
                    } else {
                        editor.tpl_editor.selected_template = 0;
                    }
                } else {
                    editor.tpl_editor.selected_template = 0;
                }
            };

            let mut cmd_mutated = false;
            match cmd {
                Cmd::SelectById(id) => {
                    if let Some(idx) = self.project.engine.rules.templates.iter().position(|t| t.id == id) {
                        self.tpl_editor.selected_template = idx;
                    }
                }
                Cmd::MoveUp { idx } => {
                    if idx > 0 {
                        let sel = selected_id_before;
                        self.project.engine.rules.templates.swap(idx, idx - 1);
                        restore(self, sel);
                        cmd_mutated = true;
                    }
                }
                Cmd::MoveDown { idx } => {
                    if idx + 1 < self.project.engine.rules.templates.len() {
                        let sel = selected_id_before;
                        self.project.engine.rules.templates.swap(idx, idx + 1);
                        restore(self, sel);
                        cmd_mutated = true;
                    }
                }
                Cmd::MoveTop { idx } => {
                    let sel = selected_id_before;
                    move_vec(&mut self.project.engine.rules.templates, idx, 0);
                    restore(self, sel);
                    cmd_mutated = true;
                }
                Cmd::MoveBottom { idx } => {
                    let n = self.project.engine.rules.templates.len();
                    if idx < n && n > 0 {
                        let sel = selected_id_before;
                        move_vec(&mut self.project.engine.rules.templates, idx, n - 1);
                        restore(self, sel);
                        cmd_mutated = true;
                    }
                }
                Cmd::SortByName => {
                    let sel = selected_id_before;
                    self.project.engine.rules.templates.sort_by(|a, b| {
                        let na = if a.name.is_empty() { &a.id } else { &a.name };
                        let nb = if b.name.is_empty() { &b.id } else { &b.name };
                        na.cmp(nb)
                    });
                    restore(self, sel);
                    cmd_mutated = true;
                }
                Cmd::Copy { idx } => {
                    if let Some(t) = self.project.engine.rules.templates.get(idx).cloned() {
                        let base_name = if t.name.is_empty() { t.id.clone() } else { t.name.clone() };
                        let mut t2 = t;
                        t2.id = self.project.engine.rules.new_template_id();
                        t2.name = format!("{}{}", base_name, s.common.copy_suffix);
                        self.project.engine.rules.templates.insert(idx + 1, t2.clone());
                        if let Some(new_idx) = self.project.engine.rules.templates.iter().position(|t| t.id == t2.id) {
                            self.tpl_editor.selected_template = new_idx;
                        }
                        cmd_mutated = true;
                    }
                }
                Cmd::Delete { idx } => {
                    let templates = &self.project.engine.rules.templates;
                    if idx < templates.len() {
                        let tid = templates[idx].id.clone();
                        if !self.project.engine.rules.is_template_referenced(&tid) {
                            self.project.engine.rules.templates.remove(idx);
                            self.tpl_editor.selected_template = self
                                .tpl_editor
                                .selected_template
                                .min(self.project.engine.rules.templates.len().saturating_sub(1));
                            cmd_mutated = true;
                        }
                    }
                }
            }
            if cmd_mutated {
                self.invalidate_render_caches();
            }
        }
    }

    fn ui_template_preview_panel(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        ui.separator();

        if self.project.engine.rules.templates.is_empty() {
            ui.weak(s.templates.no_template);
            return;
        }

        let tpl_id = self
            .project
            .engine
            .rules
            .templates
            .get(self.tpl_editor.selected_template)
            .map(|t| t.id.clone())
            .unwrap_or_default();

        let tpl_name = self
            .project
            .engine
            .rules
            .templates
            .get(self.tpl_editor.selected_template)
            .map(|t| if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() }.to_string())
            .unwrap_or_default();

        if let Some(rule_id) = self.tpl_editor.selected_variant_rule.clone() {
            let tpl = &self.project.engine.rules.templates[self.tpl_editor.selected_template];
            let rule = tpl.variant_rules.iter().find(|r| r.id == rule_id);

            if let Some(rule) = rule {
                let rule_display = if rule.name.is_empty() {
                    rule.id.as_str()
                } else {
                    rule.name.as_str()
                }
                .to_string();
                ui.horizontal(|ui| {
                    if ui.button(s.templates.back_to_all).clicked() {
                        self.tpl_editor.selected_variant_rule = None;
                    }
                    ui.label(egui::RichText::new(format!("{} {}", s.templates.rules_label, rule_display)).strong());
                });
                ui.add_space(4.0);

                let cache_key = (tpl_id.clone(), rule_id.clone());
                if self.tpl_editor.variant_cache_key.as_ref() != Some(&cache_key) {
                    self.tpl_editor.variant_cache_chars = used_chars_by_variant_rule(self, &tpl_id, &rule_id);
                    self.tpl_editor.variant_cache_triplets = used_triplets_by_variant_rule(self, &tpl_id, &rule_id);
                    self.tpl_editor.variant_cache_key = Some(cache_key);
                }
                ui.label(format!("{}{}", self.tpl_editor.variant_cache_chars.len(), s.common.char_suffix));
                ui.add_space(4.0);
                let has_archaic = self.project.old_hangul_enabled && !self.tpl_editor.variant_cache_triplets.is_empty();
                show_syllable_grid(
                    ui,
                    "tpl_variant_preview_grid",
                    &self.tpl_editor.variant_cache_chars,
                    &self.project.engine,
                    &self.project.store,
                    self.project.canvas_w,
                    self.project.canvas_h,
                    s.common.no_syllables,
                    if has_archaic { Some(ui.available_height() / 2.0) } else { None },
                );
                if has_archaic {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(s.top.old_hangul).small().strong());
                    show_jamo_triplet_grid(
                        ui,
                        "tpl_variant_archaic_grid",
                        &self.tpl_editor.variant_cache_triplets,
                        &self.project.engine,
                        &self.project.store,
                        self.project.canvas_w,
                        self.project.canvas_h,
                        s.common.no_syllables,
                    );
                }
            } else {
                self.tpl_editor.selected_variant_rule = None;
            }
        } else {
            ui.label(format!("{} {}", s.common.template_label, tpl_name));
            ui.add_space(4.0);

            if self.tpl_editor.usage_cache_key.as_deref() != Some(&tpl_id) {
                self.tpl_editor.usage_cache_chars = used_chars_by_template_id(self, &tpl_id);
                self.tpl_editor.usage_cache_triplets = used_triplets_by_template_id(self, &tpl_id);
                self.tpl_editor.usage_cache_key = Some(tpl_id.clone());
            }
            ui.label(format!("{}{}", self.tpl_editor.usage_cache_chars.len(), s.common.char_suffix));
            ui.add_space(4.0);
            let has_archaic = self.project.old_hangul_enabled && !self.tpl_editor.usage_cache_triplets.is_empty();
            show_syllable_grid(
                ui,
                "tpl_preview_grid",
                &self.tpl_editor.usage_cache_chars,
                &self.project.engine,
                &self.project.store,
                self.project.canvas_w,
                self.project.canvas_h,
                s.common.no_syllables,
                if has_archaic { Some(ui.available_height() / 2.0) } else { None },
            );
            if has_archaic {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(s.top.old_hangul).small().strong());
                show_jamo_triplet_grid(
                    ui,
                    "tpl_archaic_grid",
                    &self.tpl_editor.usage_cache_triplets,
                    &self.project.engine,
                    &self.project.store,
                    self.project.canvas_w,
                    self.project.canvas_h,
                    s.common.no_syllables,
                );
            }
        }
    }

    fn ui_template_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let s = crate::i18n::t(self.lang);
        if self.project.engine.rules.templates.is_empty() {
            ui.label(s.templates.no_template);
            return false;
        }

        let mut changed = false;

        {
            let tpl_idx = self.tpl_editor.selected_template;
            let t = &mut self.project.engine.rules.templates[tpl_idx];

            ui.heading(s.templates.settings);
            ui.separator();

            let mut name_ch = false;
            ui.horizontal(|ui| {
                ui.label(format!("{}:", s.common.name));
                name_ch = ui.text_edit_singleline(&mut t.name).changed();
            });
            if name_ch {
                changed = true;
            }
            ui.add_space(6.0);
            ui.separator();

            let old_cho = t.default_cho_group_id.clone();
            let old_jung = t.default_jung_group_id.clone();
            let old_jong = t.default_jong_group_id.clone();

            pick_group_combo(
                ui,
                ui.make_persistent_id("tpl_default_cho").with(tpl_idx),
                s.common.cho,
                HangulComponent::Cho,
                &self.project.engine.rules.groups,
                &mut self.project.engine.rules.templates[tpl_idx].default_cho_group_id,
            );
            pick_group_combo(
                ui,
                ui.make_persistent_id("tpl_default_jung").with(tpl_idx),
                s.common.jung,
                HangulComponent::Jung,
                &self.project.engine.rules.groups,
                &mut self.project.engine.rules.templates[tpl_idx].default_jung_group_id,
            );
            pick_group_combo_opt(
                ui,
                ui.make_persistent_id("tpl_default_jong").with(tpl_idx),
                s.templates.jong_none_label,
                HangulComponent::Jong,
                &self.project.engine.rules.groups,
                &mut self.project.engine.rules.templates[tpl_idx].default_jong_group_id,
                self.lang,
            );

            let t = &self.project.engine.rules.templates[tpl_idx];
            if t.default_cho_group_id != old_cho || t.default_jung_group_id != old_jung || t.default_jong_group_id != old_jong {
                changed = true;
            }

            ui.add_space(8.0);
            ui.separator();
        }

        ui.heading(s.templates.variant_rules);
        ui.small(s.templates.variant_rules_hint);

        let mut add_rule = false;
        ui.horizontal(|ui| {
            if ui.button(s.templates.add_rule).clicked() {
                add_rule = true;
            }
        });

        if add_rule {
            let new_id = self.project.engine.rules.new_variant_id();
            let t = &mut self.project.engine.rules.templates[self.tpl_editor.selected_template];
            t.variant_rules.push(VariantRule {
                id: new_id,
                name: String::new(),
                priority: 100,
                cho_set: CharSetCond::Any,
                jung_set: CharSetCond::Any,
                jong_set: CharSetCond::Any,
                set_cho_group_id: None,
                set_jung_group_id: None,
                set_jong_group_id: None,
            });
            changed = true;
        }

        let rules_snapshot: Vec<(String, String, i32)> = self.project.engine.rules.templates[self.tpl_editor.selected_template]
            .variant_rules
            .iter()
            .map(|r| (r.id.clone(), r.name.clone(), r.priority))
            .collect();

        #[derive(Clone, Debug)]
        enum RuleCmd {
            MoveUp { idx: usize },
            MoveDown { idx: usize },
            MoveTop { idx: usize },
            MoveBottom { idx: usize },
            Delete { idx: usize },
            AutoName { idx: usize },
            Copy { idx: usize },
        }

        let mut rule_cmd: Option<RuleCmd> = None;

        for (i, (rid, rname, prio)) in rules_snapshot.iter().enumerate() {
            let mut r = self.project.engine.rules.templates[self.tpl_editor.selected_template].variant_rules[i].clone();

            let display_name = if rname.is_empty() { rid.as_str() } else { rname.as_str() };
            let is_rule_selected = self.tpl_editor.selected_variant_rule.as_deref() == Some(rid.as_str());
            let header_resp = ui.add(egui::Button::new(format!("#{}  {}  (prio: {})", i, display_name, prio)).selected(is_rule_selected));
            if header_resp.clicked() {
                if is_rule_selected {
                    self.tpl_editor.selected_variant_rule = None;
                } else {
                    self.tpl_editor.selected_variant_rule = Some(rid.clone());
                }
            }

            header_resp.context_menu(|ui| {
                if ui.button(s.common.move_up).clicked() {
                    rule_cmd = Some(RuleCmd::MoveUp { idx: i });
                    ui.close();
                }
                if ui.button(s.common.move_down).clicked() {
                    rule_cmd = Some(RuleCmd::MoveDown { idx: i });
                    ui.close();
                }
                if ui.button(s.common.move_top).clicked() {
                    rule_cmd = Some(RuleCmd::MoveTop { idx: i });
                    ui.close();
                }
                if ui.button(s.common.move_bottom).clicked() {
                    rule_cmd = Some(RuleCmd::MoveBottom { idx: i });
                    ui.close();
                }
                ui.separator();
                if ui.button(s.templates.auto_name_from_cond).clicked() {
                    rule_cmd = Some(RuleCmd::AutoName { idx: i });
                    ui.close();
                }
                ui.separator();
                if ui.button(s.common.copy).clicked() {
                    rule_cmd = Some(RuleCmd::Copy { idx: i });
                    ui.close();
                }
                ui.separator();
                if ui.button(s.common.delete).clicked() {
                    rule_cmd = Some(RuleCmd::Delete { idx: i });
                    ui.close();
                }
            });

            let mut rule_ch = false;
            ui.group(|ui| {
                let mut widget_ch = false;
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", s.common.name));
                    if ui.text_edit_singleline(&mut r.name).changed() {
                        widget_ch = true;
                    }
                    ui.label(s.selectors.priority);
                    if ui.add(egui::DragValue::new(&mut r.priority).range(-10000..=10000)).changed() {
                        widget_ch = true;
                    }
                });
                if widget_ch {
                    rule_ch = true;
                }
                ui_separator_soft(ui);

                let old_cho_gid = r.set_cho_group_id.clone();
                let old_jung_gid = r.set_jung_group_id.clone();
                let old_jong_gid = r.set_jong_group_id.clone();
                pick_group_combo_opt(
                    ui,
                    ui.make_persistent_id("vr_set_cho").with((self.tpl_editor.selected_template, i)),
                    s.templates.set_cho_group,
                    HangulComponent::Cho,
                    &self.project.engine.rules.groups,
                    &mut r.set_cho_group_id,
                    self.lang,
                );
                pick_group_combo_opt(
                    ui,
                    ui.make_persistent_id("vr_set_jung").with((self.tpl_editor.selected_template, i)),
                    s.templates.set_jung_group,
                    HangulComponent::Jung,
                    &self.project.engine.rules.groups,
                    &mut r.set_jung_group_id,
                    self.lang,
                );
                pick_group_combo_opt(
                    ui,
                    ui.make_persistent_id("vr_set_jong").with((self.tpl_editor.selected_template, i)),
                    s.templates.set_jong_group,
                    HangulComponent::Jong,
                    &self.project.engine.rules.groups,
                    &mut r.set_jong_group_id,
                    self.lang,
                );
                if r.set_cho_group_id != old_cho_gid || r.set_jung_group_id != old_jung_gid || r.set_jong_group_id != old_jong_gid {
                    rule_ch = true;
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
                    jong_ext = jong_allowed_ext();
                    &jong_ext
                } else {
                    jong_allowed()
                };
                ui_charset_toggle(
                    ui,
                    ui.make_persistent_id("vr_cho_set").with((self.tpl_editor.selected_template, i)),
                    s.templates.cho_filter,
                    &mut r.cho_set,
                    cho_set,
                    10,
                    self.lang,
                );
                ui_charset_toggle(
                    ui,
                    ui.make_persistent_id("vr_jung_set").with((self.tpl_editor.selected_template, i)),
                    s.templates.jung_filter,
                    &mut r.jung_set,
                    jung_set,
                    10,
                    self.lang,
                );
                ui_charset_toggle(
                    ui,
                    ui.make_persistent_id("vr_jong_set").with((self.tpl_editor.selected_template, i)),
                    s.templates.jong_filter,
                    &mut r.jong_set,
                    jong_set,
                    10,
                    self.lang,
                );
                if r.cho_set != old_cho_set || r.jung_set != old_jung_set || r.jong_set != old_jong_set {
                    rule_ch = true;
                }
            });

            self.project.engine.rules.templates[self.tpl_editor.selected_template].variant_rules[i] = r;
            if rule_ch {
                changed = true;
            }
            ui.add_space(6.0);
        }

        let pre_copy_id = if matches!(rule_cmd, Some(RuleCmd::Copy { .. })) {
            Some(self.project.engine.rules.new_variant_id())
        } else {
            None
        };
        let mut set_selected_rule: Option<String> = None;

        if let Some(cmd) = rule_cmd {
            let vr = &mut self.project.engine.rules.templates[self.tpl_editor.selected_template].variant_rules;
            match cmd {
                RuleCmd::MoveUp { idx } => {
                    if idx > 0 && idx < vr.len() {
                        vr.swap(idx, idx - 1);
                    }
                }
                RuleCmd::MoveDown { idx } => {
                    if idx + 1 < vr.len() {
                        vr.swap(idx, idx + 1);
                    }
                }
                RuleCmd::MoveTop { idx } => {
                    if idx < vr.len() {
                        move_vec(vr, idx, 0);
                    }
                }
                RuleCmd::MoveBottom { idx } => {
                    let n = vr.len();
                    if idx < n && n > 0 {
                        move_vec(vr, idx, n - 1);
                    }
                }
                RuleCmd::Delete { idx } => {
                    if idx < vr.len() {
                        vr.remove(idx);
                    }
                }
                RuleCmd::AutoName { idx } => {
                    if idx < vr.len() {
                        let new_name = variant_rule_auto_name(&vr[idx], s.common.cho, s.common.jung, s.common.jong);
                        vr[idx].name = new_name;
                    }
                }
                RuleCmd::Copy { idx } => {
                    if idx < vr.len() {
                        let mut r2 = vr[idx].clone();
                        let new_id = pre_copy_id.unwrap();
                        let base_name = if r2.name.is_empty() { r2.id.clone() } else { r2.name.clone() };
                        r2.id = new_id.clone();
                        r2.name = format!("{}{}", base_name, s.common.copy_suffix);
                        vr.insert(idx + 1, r2);
                        set_selected_rule = Some(new_id);
                    }
                }
            }
            ui.ctx().request_repaint();
            changed = true;
        }

        if let Some(id) = set_selected_rule {
            self.tpl_editor.selected_variant_rule = Some(id);
        }

        changed
    }
}

pub fn pick_group_combo_opt(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    target: HangulComponent,
    groups: &[hangul_syllable::core::groups::ComponentGroup],
    v: &mut Option<String>,
    lang: crate::i18n::Lang,
) {
    let none_text = crate::i18n::t(lang).common.none;
    ui.horizontal(|ui| {
        ui.label(label);

        let selected_text = match v.as_deref() {
            None => none_text.to_string(),
            Some(gid) => groups
                .iter()
                .find(|g| g.id == gid)
                .map(|g| g.name.to_string())
                .unwrap_or_else(|| format!("(missing) {}", gid)),
        };

        egui::ComboBox::from_id_salt(id).selected_text(selected_text).show_ui(ui, |ui| {
            ui.selectable_value(v, None, none_text);
            for g in groups.iter().filter(|g| g.target == target) {
                ui.selectable_value(v, Some(g.id.clone()), g.name.to_string());
            }
        });
    });
}

pub(super) fn used_chars_by_variant_rule(editor: &crate::app::editor::FontEditor, template_id: &str, rule_id: &str) -> Vec<char> {
    let Some(tpl) = editor.project.engine.rules.find_template(template_id) else {
        return vec![];
    };
    let Some(rule_idx) = tpl.variant_rules.iter().position(|r| r.id == rule_id) else {
        return vec![];
    };
    let tpl = tpl.clone();

    let mut out = Vec::new();
    for s in 0xAC00u32..=0xD7A3u32 {
        let Some(ch) = char::from_u32(s) else { continue };
        let Some(layout) = editor
            .project
            .engine
            .layout_char(&editor.project.store, ch, decompose_hangul, get_jamo_char)
        else {
            continue;
        };
        if layout.template_id != template_id {
            continue;
        }

        let cho_ch = layout.cho.placement.jamo;
        let jung_ch = layout.jung.placement.jamo;
        let jong_ch = layout.jong.as_ref().map(|j| j.placement.jamo);
        let default_cho = &tpl.default_cho_group_id;
        let default_jung = &tpl.default_jung_group_id;
        let default_jong = tpl.default_jong_group_id.as_deref().unwrap_or("");

        let with_rule = sim_variant_rules(
            tpl.variant_rules.iter(),
            cho_ch,
            jung_ch,
            jong_ch,
            default_cho,
            default_jung,
            default_jong,
        );
        let without_rule = sim_variant_rules(
            tpl.variant_rules.iter().enumerate().filter(|(i, _)| *i != rule_idx).map(|(_, r)| r),
            cho_ch,
            jung_ch,
            jong_ch,
            default_cho,
            default_jung,
            default_jong,
        );

        if with_rule != without_rule {
            out.push(ch);
        }
    }
    out
}

fn sim_variant_rules<'a>(
    rules: impl Iterator<Item = &'a VariantRule>,
    cho_ch: char,
    jung_ch: char,
    jong_ch: Option<char>,
    default_cho: &str,
    default_jung: &str,
    default_jong: &str,
) -> (String, String, String) {
    let mut rules: Vec<&VariantRule> = rules.collect();
    rules.sort_unstable_by_key(|r| r.priority);

    let mut cho = default_cho.to_string();
    let mut jung = default_jung.to_string();
    let mut jong = default_jong.to_string();
    let jong_for_match = jong_ch.unwrap_or(NO_JONG);

    for r in rules {
        if r.cho_set.matches(cho_ch) && r.jung_set.matches(jung_ch) && r.jong_set.matches(jong_for_match) {
            if let Some(gid) = &r.set_cho_group_id {
                cho = gid.clone();
            }
            if let Some(gid) = &r.set_jung_group_id {
                jung = gid.clone();
            }
            if let Some(gid) = &r.set_jong_group_id {
                jong = gid.clone();
            }
        }
    }

    (cho, jung, jong)
}

pub(super) fn used_triplets_by_template_id(editor: &crate::app::editor::FontEditor, template_id: &str) -> Vec<JamoTriplet> {
    if !editor.project.old_hangul_enabled {
        return vec![];
    }

    let modern_cho = cho_allowed();
    let modern_jung = jung_allowed();
    let modern_jong = jong_allowed();

    let archaic_cho: Vec<char> = cho_allowed_ext().into_iter().filter(|c| !modern_cho.contains(c)).collect();
    let archaic_jung: Vec<char> = jung_allowed_ext().into_iter().filter(|c| !modern_jung.contains(c)).collect();
    let archaic_jong: Vec<char> = jong_allowed_ext().into_iter().filter(|c| !modern_jong.contains(c)).collect();

    let ref_cho = modern_cho[0];
    let ref_jung = modern_jung[0];
    let ref_jong = modern_jong[0];

    let try_triplet = |cho: char, jung: char, jong: Option<char>| -> Option<JamoTriplet> {
        editor
            .project
            .engine
            .layout_jamo(&editor.project.store, cho, jung, jong)
            .filter(|l| l.template_id == template_id)
            .map(|_| (cho, jung, jong))
    };

    let mut out = Vec::new();

    for c in archaic_cho {
        for &j in modern_jung {
            if let Some(t) = try_triplet(c, j, None) {
                out.push(t);
            }
            if let Some(t) = try_triplet(c, j, Some(ref_jong)) {
                out.push(t);
            }
        }
    }
    for j in archaic_jung {
        for &c in modern_cho {
            if let Some(t) = try_triplet(c, j, None) {
                out.push(t);
            }
            if let Some(t) = try_triplet(c, j, Some(ref_jong)) {
                out.push(t);
            }
        }
    }
    for k in archaic_jong {
        if let Some(t) = try_triplet(ref_cho, ref_jung, Some(k)) {
            out.push(t);
        }
    }

    out
}

pub(super) fn used_triplets_by_variant_rule(editor: &crate::app::editor::FontEditor, template_id: &str, rule_id: &str) -> Vec<JamoTriplet> {
    if !editor.project.old_hangul_enabled {
        return vec![];
    }

    let Some(tpl) = editor.project.engine.rules.find_template(template_id) else {
        return vec![];
    };
    let Some(rule_idx) = tpl.variant_rules.iter().position(|r| r.id == rule_id) else {
        return vec![];
    };
    let tpl = tpl.clone();

    let default_cho = tpl.default_cho_group_id.clone();
    let default_jung = tpl.default_jung_group_id.clone();
    let default_jong_str = tpl.default_jong_group_id.as_deref().unwrap_or("").to_string();

    let modern_cho = cho_allowed();
    let modern_jung = jung_allowed();
    let modern_jong = jong_allowed();

    let archaic_cho: Vec<char> = cho_allowed_ext().into_iter().filter(|c| !modern_cho.contains(c)).collect();
    let archaic_jung: Vec<char> = jung_allowed_ext().into_iter().filter(|c| !modern_jung.contains(c)).collect();
    let archaic_jong: Vec<char> = jong_allowed_ext().into_iter().filter(|c| !modern_jong.contains(c)).collect();

    let ref_cho = modern_cho[0];
    let ref_jung = modern_jung[0];
    let ref_jong = modern_jong[0];

    let try_triplet = |cho: char, jung: char, jong: Option<char>| -> Option<JamoTriplet> {
        editor.project.engine.layout_jamo(&editor.project.store, cho, jung, jong)?;
        let with_rule = sim_variant_rules(
            tpl.variant_rules.iter(),
            cho,
            jung,
            jong,
            &default_cho,
            &default_jung,
            &default_jong_str,
        );
        let without_rule = sim_variant_rules(
            tpl.variant_rules.iter().enumerate().filter(|(i, _)| *i != rule_idx).map(|(_, r)| r),
            cho,
            jung,
            jong,
            &default_cho,
            &default_jung,
            &default_jong_str,
        );
        if with_rule != without_rule { Some((cho, jung, jong)) } else { None }
    };

    let mut out = Vec::new();

    for c in archaic_cho {
        for &j in modern_jung {
            if let Some(t) = try_triplet(c, j, None) {
                out.push(t);
            }
            if let Some(t) = try_triplet(c, j, Some(ref_jong)) {
                out.push(t);
            }
        }
    }
    for j in archaic_jung {
        for &c in modern_cho {
            if let Some(t) = try_triplet(c, j, None) {
                out.push(t);
            }
            if let Some(t) = try_triplet(c, j, Some(ref_jong)) {
                out.push(t);
            }
        }
    }
    for k in archaic_jong {
        if let Some(t) = try_triplet(ref_cho, ref_jung, Some(k)) {
            out.push(t);
        }
    }

    out
}

fn variant_rule_auto_name(rule: &VariantRule, cho_label: &str, jung_label: &str, jong_label: &str) -> String {
    let fmt_cond = |label: &str, cond: &CharSetCond| -> Option<String> {
        match cond {
            CharSetCond::Any => None,
            CharSetCond::Include(set) => {
                let chars: String = set.iter().map(|&c| if c == NO_JONG { '-' } else { c }).collect();
                Some(format!("{}[{}]", label, chars))
            }
            CharSetCond::Exclude(set) => {
                let chars: String = set.iter().map(|&c| if c == NO_JONG { '-' } else { c }).collect();
                Some(format!("{}[^{}]", label, chars))
            }
        }
    };

    let mut parts = Vec::new();
    if let Some(s) = fmt_cond(cho_label, &rule.cho_set) {
        parts.push(s);
    }
    if let Some(s) = fmt_cond(jung_label, &rule.jung_set) {
        parts.push(s);
    }
    if let Some(s) = fmt_cond(jong_label, &rule.jong_set) {
        parts.push(s);
    }

    if parts.is_empty() { String::from("*") } else { parts.join("+") }
}

pub(super) fn used_chars_by_template_id(editor: &crate::app::editor::FontEditor, template_id: &str) -> Vec<char> {
    let mut out = Vec::new();
    for s in 0xAC00u32..=0xD7A3u32 {
        let Some(ch) = char::from_u32(s) else { continue };
        let Some(layout) = editor
            .project
            .engine
            .layout_char(&editor.project.store, ch, decompose_hangul, get_jamo_char)
        else {
            continue;
        };
        if layout.template_id == template_id {
            out.push(ch);
        }
    }
    out
}
