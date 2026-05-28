use eframe::egui;

use crate::app::{
    editor::FontEditor,
    ui_widgets::{JamoTriplet, move_vec, show_jamo_triplet_grid, show_syllable_grid},
};
use hangul_syllable::core::hangul::{allowed_chars_extended, allowed_chars_for_target, decompose_hangul, get_jamo_char};
use hangul_syllable::{GlyphKey, GroupRef, HangulComponent};

impl FontEditor {
    pub fn ui_components_mode(&mut self, ui: &mut egui::Ui) {
        use crate::app::ui_widgets::sub_panel_tabs;

        if ui.available_width() < crate::app::editor::NARROW_WIDTH {
            let s = crate::i18n::t(self.lang);
            sub_panel_tabs(ui, &mut self.narrow_comp_sub, &[s.common.list, s.common.edit, s.common.preview]);
            ui.separator();
            match self.narrow_comp_sub {
                0 => self.render_group_list_panel(ui),
                1 => self.render_group_editor_panel(ui),
                2 => self.ui_component_usage_preview(ui),
                _ => {}
            }
            return;
        }

        egui::Panel::left("comp_left")
            .min_size(320.0)
            .max_size(600.0)
            .show_inside(ui, |ui| {
                self.render_group_list_panel(ui);
            });

        egui::Panel::right("comp_right_usage")
            .min_size(300.0)
            .max_size(600.0)
            .show_inside(ui, |ui| {
                self.ui_component_usage_preview(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_group_editor_panel(ui);
        });
    }

    fn render_group_list_panel(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        egui::Panel::bottom("left_bottom_new_group").resizable(false).show_inside(ui, |ui| {
            ui.heading(s.components.new_group);

            ui.horizontal(|ui| {
                ui.label(format!("{}:", s.common.name));
                ui.text_edit_singleline(&mut self.new_group.name);
            });

            ui.horizontal(|ui| {
                ui.label(s.components.target);
                let target_text = match self.new_group.target {
                    HangulComponent::Cho => s.common.cho,
                    HangulComponent::Jung => s.common.jung,
                    HangulComponent::Jong => s.common.jong,
                };
                egui::ComboBox::from_id_salt("new_group_target_combo")
                    .selected_text(target_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_group.target, HangulComponent::Cho, s.common.cho);
                        ui.selectable_value(&mut self.new_group.target, HangulComponent::Jung, s.common.jung);
                        ui.selectable_value(&mut self.new_group.target, HangulComponent::Jong, s.common.jong);
                    });
            });

            ui.separator();
            if ui.button(s.components.create_group_btn).clicked() {
                let ext_chars: Vec<char>;
                let allowed: &[char] = if self.project.old_hangul_enabled {
                    ext_chars = allowed_chars_extended(self.new_group.target);
                    &ext_chars
                } else {
                    allowed_chars_for_target(self.new_group.target)
                };
                self.project.engine.rules.add_group(
                    &mut self.project.store,
                    self.new_group.name.trim(),
                    self.new_group.target,
                    allowed.iter().copied().collect(),
                );
                self.components.selected_group = self.project.engine.rules.groups.len().saturating_sub(1);
                self.components.selected_member_index = None;
                self.invalidate_render_caches();
            }
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.components.group_filter, None, s.components.all);
                ui.selectable_value(&mut self.components.group_filter, Some(HangulComponent::Cho), s.common.cho);
                ui.selectable_value(&mut self.components.group_filter, Some(HangulComponent::Jung), s.common.jung);
                ui.selectable_value(&mut self.components.group_filter, Some(HangulComponent::Jong), s.common.jong);
            });
            ui.separator();

            let groups_snapshot: Vec<(usize, String, String, HangulComponent)> = self
                .project
                .engine
                .rules
                .groups
                .iter()
                .enumerate()
                .filter(|(_, g)| self.components.group_filter.is_none_or(|target| g.target == target))
                .map(|(orig_idx, g)| (orig_idx, g.id.clone(), g.name.clone(), g.target))
                .collect();

            let selected_id_before = self
                .project
                .engine
                .rules
                .groups
                .get(self.components.selected_group)
                .map(|g| g.id.clone());

            #[derive(Clone, Debug)]
            enum Cmd {
                MoveTop { idx: usize },
                MoveBottom { idx: usize },
                MoveUp { idx: usize },
                MoveDown { idx: usize },
                SortByName,
                SortByTargetThenName,
                SelectById(String),
                DeleteGroup { gid: String },
                CopyGroup { gid: String },
            }

            let mut cmd: Option<Cmd> = None;

            let can_delete = selected_id_before
                .as_ref()
                .map(|gid| !self.project.engine.rules.is_group_referenced(gid))
                .unwrap_or(false);

            ui.horizontal(|ui| {
                if ui.button(s.common.add).clicked() {
                    let target = self.components.group_filter.unwrap_or(HangulComponent::Cho);
                    let ext_chars: Vec<char>;
                    let allowed: &[char] = if self.project.old_hangul_enabled {
                        ext_chars = allowed_chars_extended(target);
                        &ext_chars
                    } else {
                        allowed_chars_for_target(target)
                    };
                    self.project.engine.rules.add_group(
                        &mut self.project.store,
                        s.components.new_group_default_name,
                        target,
                        allowed.iter().copied().collect(),
                    );
                    self.components.selected_group = self.project.engine.rules.groups.len().saturating_sub(1);
                    self.components.selected_member_index = None;
                    self.invalidate_render_caches();
                }
                ui.add_enabled_ui(!self.project.engine.rules.groups.is_empty(), |ui| {
                    if ui.button(s.common.copy).clicked()
                        && let Some(gid) = selected_id_before.clone()
                    {
                        cmd = Some(Cmd::CopyGroup { gid });
                    }
                    ui.add_enabled_ui(can_delete, |ui| {
                        if ui.button(s.common.delete).clicked()
                            && let Some(gid) = selected_id_before.clone()
                        {
                            cmd = Some(Cmd::DeleteGroup { gid });
                        }
                    });
                });
            });

            egui::ScrollArea::vertical()
                .id_salt("groups_list_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (orig_idx, gid, name, target) in groups_snapshot.iter() {
                        let selected = *orig_idx == self.components.selected_group;
                        let label = format!(
                            "{} ({})",
                            name,
                            match target {
                                HangulComponent::Cho => s.common.cho,
                                HangulComponent::Jung => s.common.jung,
                                HangulComponent::Jong => s.common.jong,
                            }
                        );

                        let resp = ui.add(egui::Button::new(label).selected(selected));

                        if resp.clicked() {
                            cmd = Some(Cmd::SelectById(gid.clone()));
                        }

                        resp.context_menu(|ui| {
                            if ui.button(s.common.move_up).clicked() {
                                cmd = Some(Cmd::MoveUp { idx: *orig_idx });
                                ui.close();
                            }
                            if ui.button(s.common.move_down).clicked() {
                                cmd = Some(Cmd::MoveDown { idx: *orig_idx });
                                ui.close();
                            }
                            if ui.button(s.common.move_top).clicked() {
                                cmd = Some(Cmd::MoveTop { idx: *orig_idx });
                                ui.close();
                            }
                            if ui.button(s.common.move_bottom).clicked() {
                                cmd = Some(Cmd::MoveBottom { idx: *orig_idx });
                                ui.close();
                            }

                            ui.separator();
                            if ui.button(s.common.sort_by_name).clicked() {
                                cmd = Some(Cmd::SortByName);
                                ui.close();
                            }
                            if ui.button(s.components.sort_by_target_name).clicked() {
                                cmd = Some(Cmd::SortByTargetThenName);
                                ui.close();
                            }

                            ui.separator();
                            if ui.button(s.common.copy).clicked() {
                                cmd = Some(Cmd::CopyGroup { gid: gid.clone() });
                                ui.close();
                            }

                            ui.separator();
                            let referenced = self.project.engine.rules.is_group_referenced(gid);
                            ui.add_enabled_ui(!referenced, |ui| {
                                if ui.button(s.common.delete).clicked() {
                                    cmd = Some(Cmd::DeleteGroup { gid: gid.clone() });
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
                match cmd {
                    Cmd::SelectById(gid) => {
                        if let Some(idx) = self.project.engine.rules.groups.iter().position(|g| g.id == gid) {
                            self.components.selected_group = idx;
                            self.components.selected_member_index = None;
                        }
                    }
                    Cmd::MoveUp { idx } => {
                        if idx > 0 {
                            let sel_id = selected_id_before;
                            self.project.engine.rules.groups.swap(idx, idx - 1);
                            if let Some(id) = sel_id
                                && let Some(new_idx) = self.project.engine.rules.groups.iter().position(|g| g.id == id)
                            {
                                self.components.selected_group = new_idx;
                            }
                        }
                    }
                    Cmd::MoveDown { idx } => {
                        if idx + 1 < self.project.engine.rules.groups.len() {
                            let sel_id = selected_id_before;
                            self.project.engine.rules.groups.swap(idx, idx + 1);
                            if let Some(id) = sel_id
                                && let Some(new_idx) = self.project.engine.rules.groups.iter().position(|g| g.id == id)
                            {
                                self.components.selected_group = new_idx;
                            }
                        }
                    }
                    Cmd::MoveTop { idx } => {
                        if idx < self.project.engine.rules.groups.len() {
                            let sel_id = selected_id_before;
                            move_vec(&mut self.project.engine.rules.groups, idx, 0);
                            if let Some(id) = sel_id
                                && let Some(new_idx) = self.project.engine.rules.groups.iter().position(|g| g.id == id)
                            {
                                self.components.selected_group = new_idx;
                            }
                        }
                    }
                    Cmd::MoveBottom { idx } => {
                        let n = self.project.engine.rules.groups.len();
                        if idx < n && n > 0 {
                            let sel_id = selected_id_before;
                            move_vec(&mut self.project.engine.rules.groups, idx, n - 1);
                            if let Some(id) = sel_id
                                && let Some(new_idx) = self.project.engine.rules.groups.iter().position(|g| g.id == id)
                            {
                                self.components.selected_group = new_idx;
                            }
                        }
                    }
                    Cmd::SortByName => {
                        let sel_id = selected_id_before;
                        self.project.engine.rules.groups.sort_by(|a, b| a.name.cmp(&b.name));
                        if let Some(id) = sel_id {
                            self.components.selected_group = self.project.engine.rules.groups.iter().position(|g| g.id == id).unwrap_or(0);
                        }
                    }
                    Cmd::SortByTargetThenName => {
                        let sel_id = selected_id_before;
                        self.project.engine.rules.groups.sort_by(|a, b| {
                            let ta = a.target as u8;
                            let tb = b.target as u8;
                            ta.cmp(&tb).then(a.name.cmp(&b.name))
                        });
                        if let Some(id) = sel_id {
                            self.components.selected_group = self.project.engine.rules.groups.iter().position(|g| g.id == id).unwrap_or(0);
                        }
                    }
                    Cmd::DeleteGroup { gid } => {
                        {
                            let rules = &mut self.project.engine.rules;
                            let target_opt = rules.find_group(&gid).map(|g| g.target);
                            if let Some(t) = target_opt {
                                let next_id = rules
                                    .groups
                                    .iter()
                                    .find(|g| g.target == t && g.id != gid)
                                    .map(|g| g.id.clone())
                                    .unwrap_or_default();
                                match t {
                                    HangulComponent::Cho if rules.base_cho_group_id == gid => rules.base_cho_group_id = next_id,
                                    HangulComponent::Jung if rules.base_jung_group_id == gid => rules.base_jung_group_id = next_id,
                                    HangulComponent::Jong if rules.base_jong_group_id == gid => rules.base_jong_group_id = next_id,
                                    _ => {}
                                }
                            }
                        }

                        self.project.store.remove_group_glyphs(&gid);
                        self.project.engine.rules.groups.retain(|g| g.id != gid);

                        if self.project.engine.rules.groups.is_empty() {
                            self.components.selected_group = 0;
                            self.components.selected_member_index = None;
                        } else if let Some(id) = selected_id_before {
                            if let Some(new_idx) = self.project.engine.rules.groups.iter().position(|g| g.id == id) {
                                self.components.selected_group = new_idx;
                            } else {
                                self.components.selected_group =
                                    self.components.selected_group.min(self.project.engine.rules.groups.len() - 1);
                                self.components.selected_member_index = None;
                            }
                        } else {
                            self.components.selected_group = self.components.selected_group.min(self.project.engine.rules.groups.len() - 1);
                            self.components.selected_member_index = None;
                        }
                        self.invalidate_render_caches();
                    }
                    Cmd::CopyGroup { gid } => {
                        if let Some(src) = self.project.engine.rules.find_group(&gid).cloned() {
                            let new_id = self.project.engine.rules.new_group_id();
                            let mut g2 = src;
                            g2.id = new_id.clone();
                            g2.name = format!("{} (copy)", g2.name);
                            self.project.engine.rules.groups.push(g2);
                            self.project.store.clone_group_glyphs(&gid, &new_id);
                            self.components.selected_group = self.project.engine.rules.groups.len().saturating_sub(1);
                            self.components.selected_member_index = None;
                            self.invalidate_render_caches();
                        }
                    }
                }
            }
        });
    }

    fn render_group_editor_panel(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);

        if self.project.engine.rules.groups.is_empty() {
            ui.label(s.components.no_groups);
            return;
        }

        let group_idx = self.components.selected_group.min(self.project.engine.rules.groups.len() - 1);
        let group_id = self.project.engine.rules.groups[group_idx].id.clone();
        let kind = self.project.engine.rules.groups[group_idx].target;

        ui.heading(format!(
            "{} {}",
            s.components.edit_group, self.project.engine.rules.groups[group_idx].name
        ));
        ui.separator();

        {
            let g = &mut self.project.engine.rules.groups[group_idx];
            ui.horizontal(|ui| {
                ui.label(format!("{}:", s.common.name));
                if ui.text_edit_singleline(&mut g.name).changed() {
                    self.project.is_dirty = true;
                }
            });
        }

        ui.separator();

        let ext_chars: Vec<char>;
        let allowed: &[char] = if self.project.old_hangul_enabled {
            ext_chars = allowed_chars_extended(kind);
            &ext_chars
        } else {
            allowed_chars_for_target(kind)
        };
        egui::ScrollArea::vertical()
            .id_salt(format!("edit_members_scroll_{}", group_id))
            .max_height(150.0)
            .show(ui, |ui| {
                egui::Grid::new(format!("edit_members_grid_{}", group_id))
                    .spacing([4.0, 4.0])
                    .show(ui, |ui| {
                        for (i, ch) in allowed.iter().copied().enumerate() {
                            let is_on = self.project.engine.rules.groups[group_idx].members.contains(&ch);
                            if ui.selectable_label(is_on, ch.to_string()).clicked() {
                                if is_on {
                                    self.project.engine.rules.groups[group_idx].members.remove(&ch);
                                } else {
                                    self.project.engine.rules.groups[group_idx].members.insert(ch);
                                }
                                self.invalidate_render_caches();
                            }
                            if (i + 1) % 15 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

        ui.horizontal(|ui| {
            if ui.button(s.components.select_all).clicked() {
                self.project.engine.rules.groups[group_idx].members = allowed.iter().copied().collect();
                self.invalidate_render_caches();
            }
            if ui.button(s.components.deselect_all).clicked() {
                self.project.engine.rules.groups[group_idx].members.clear();
                self.invalidate_render_caches();
            }
        });

        ui.add_space(10.0);
        ui.separator();

        let members_sorted: Vec<char> = self.project.engine.rules.groups[group_idx].members.iter().copied().collect();
        let thumb = 36.0;
        let cols = 10;

        #[derive(Clone)]
        enum PixelCmd {
            Copy(GlyphKey),
            Paste(GlyphKey),
        }
        let mut pixel_cmd: Option<PixelCmd> = None;

        egui::ScrollArea::vertical()
            .id_salt(format!("thumb_scroll_{}", group_id))
            .max_height(250.0)
            .show(ui, |ui| {
                egui::Grid::new(format!("thumb_grid_{}", group_id))
                    .spacing([8.0, 12.0])
                    .show(ui, |ui| {
                        for (i, ch) in members_sorted.iter().copied().enumerate() {
                            ui.push_id(ch as u32, |ui| {
                                ui.vertical_centered(|ui| {
                                    self.project.store.ensure_glyph(GlyphKey::new(kind, ch, &group_id));

                                    let key = GlyphKey::new(kind, ch, group_id.clone());
                                    let is_selected = self.components.selected_member_index == Some(ch);

                                    let resp = crate::app::ui_widgets::draw_glyph_thumbnail(
                                        ui,
                                        &self.project.store,
                                        &key,
                                        thumb,
                                        self.project.canvas_w,
                                        self.project.canvas_h,
                                    );

                                    if is_selected {
                                        ui.painter().rect_stroke(
                                            resp.rect.expand(2.0),
                                            2.0,
                                            egui::Stroke::new(2.2, egui::Color32::from_rgb(255, 215, 0)),
                                            egui::StrokeKind::Outside,
                                        );
                                    }

                                    ui.small(ch.to_string());

                                    if resp.clicked() {
                                        self.components.selected_member_index = Some(ch);
                                    }

                                    let has_clipboard = self.drawing.pixel_clipboard.is_some();
                                    resp.context_menu(|ui| {
                                        let s = crate::i18n::t(self.lang);
                                        if ui.button(s.common.copy).clicked() {
                                            pixel_cmd = Some(PixelCmd::Copy(key.clone()));
                                            ui.close();
                                        }
                                        ui.add_enabled_ui(has_clipboard, |ui| {
                                            if ui.button(s.common.paste).clicked() {
                                                pixel_cmd = Some(PixelCmd::Paste(key.clone()));
                                                ui.close();
                                            }
                                        });
                                    });
                                });
                            });

                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

        if let Some(cmd) = pixel_cmd {
            match cmd {
                PixelCmd::Copy(k) => self.copy_pixels_from(&k),
                PixelCmd::Paste(k) => self.paste_pixels_to(&k),
            }
        }

        ui.add_space(10.0);

        if let Some(ch) = self.components.selected_member_index {
            if self.project.engine.rules.groups[group_idx].members.contains(&ch) {
                let key = GlyphKey::new(kind, ch, group_id.clone());

                ui.horizontal(|ui| {
                    ui.label(s.components.adjust_position);
                    if ui.button("⏴").on_hover_text(s.components.move_left).clicked() {
                        if let Some(g) = self.project.store.get_mut(&key) {
                            g.shift(-1, 0);
                        }
                        self.invalidate_render_caches();
                    }
                    if ui.button("⏵").on_hover_text(s.components.move_right).clicked() {
                        if let Some(g) = self.project.store.get_mut(&key) {
                            g.shift(1, 0);
                        }
                        self.invalidate_render_caches();
                    }
                    if ui.button("⏶").on_hover_text(s.components.move_up).clicked() {
                        if let Some(g) = self.project.store.get_mut(&key) {
                            g.shift(0, -1);
                        }
                        self.invalidate_render_caches();
                    }
                    if ui.button("⏷").on_hover_text(s.components.move_down).clicked() {
                        if let Some(g) = self.project.store.get_mut(&key) {
                            g.shift(0, 1);
                        }
                        self.invalidate_render_caches();
                    }

                    ui.separator();
                    if ui.button(s.components.clear).clicked() {
                        if let Some(g) = self.project.store.get_mut(&key) {
                            g.pixels.clear();
                        }
                        self.invalidate_render_caches();
                    }
                });
                ui.add_space(4.0);

                self.render_local_pixel_editor(ui, key);
            } else {
                self.components.selected_member_index = None;
            }
        } else {
            ui.label(s.components.select_member);
        }
    }

    fn ui_component_usage_preview(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);
        ui.separator();

        if self.project.engine.rules.groups.is_empty() {
            ui.label(s.components.no_group);
            return;
        }
        let n = self.project.engine.rules.groups.len();
        if self.components.selected_group >= n {
            self.components.selected_group = n - 1;
        }

        let group_idx = self.components.selected_group;
        let group_id = self.project.engine.rules.groups[group_idx].id.clone();
        let target = self.project.engine.rules.groups[group_idx].target;

        if self.project.old_hangul_enabled {
            if let Some(ch) = self.components.selected_member_index {
                let key = (group_id.clone(), target, ch);
                if self.components.archaic_cache_key.as_ref() != Some(&key) {
                    self.components.archaic_cache_triplets = archaic_triplets_for_component_in_group(self, target, ch, &group_id);
                    self.components.archaic_cache_key = Some(key);
                }
                self.components.archaic_group_cache_key = None;
            } else {
                if self.components.archaic_group_cache_key.as_deref() != Some(group_id.as_str()) {
                    self.components.archaic_group_cache_triplets = archaic_triplets_for_group(self, &group_id);
                    self.components.archaic_group_cache_key = Some(group_id.clone());
                }
                self.components.archaic_cache_key = None;
                self.components.archaic_cache_triplets.clear();
            }
        }

        let group = &self.project.engine.rules.groups[group_idx];

        let refs = self.project.engine.rules.collect_group_refs(&group_id);
        egui::CollapsingHeader::new(s.components.backrefs_heading)
            .default_open(true)
            .show(ui, |ui| {
                render_group_backrefs(ui, &refs, s);
            });
        ui.separator();

        if let Some(ch) = self.components.selected_member_index {
            if !group.members.contains(&ch) {
                ui.label(s.components.member_not_in_group);
                return;
            }
            ui.label(format!("{} {}", s.components.selected_component, ch));

            let key = (group_id.clone(), target, ch);
            if self.components.usage_cache_key.as_ref() != Some(&key) {
                self.components.usage_cache_chars = used_chars_using_component(self, target, ch, &group_id);
                self.components.usage_cache_key = Some(key);
            }
            let has_archaic = !self.components.archaic_cache_triplets.is_empty();
            show_syllable_grid(
                ui,
                "comp_member_preview",
                &self.components.usage_cache_chars,
                &self.project.engine,
                &self.project.store,
                self.project.canvas_w,
                self.project.canvas_h,
                s.common.no_syllables,
                if has_archaic { Some(ui.available_height() / 2.0) } else { None },
            );

            if has_archaic {
                ui.separator();
                ui.small(s.top.old_hangul);
                show_jamo_triplet_grid(
                    ui,
                    "comp_archaic_member",
                    &self.components.archaic_cache_triplets,
                    &self.project.engine,
                    &self.project.store,
                    self.project.canvas_w,
                    self.project.canvas_h,
                    s.common.no_syllables,
                );
            }
        } else {
            let group_name = group.name.clone();
            ui.label(format!("{} {}", s.components.group_total, group_name));

            if self.components.group_cache_key.as_deref() != Some(&group_id) {
                self.components.group_cache_chars = used_chars_using_group(self, &group_id);
                self.components.group_cache_key = Some(group_id);
            }
            let has_archaic_group = self.project.old_hangul_enabled && !self.components.archaic_group_cache_triplets.is_empty();
            show_syllable_grid(
                ui,
                "comp_group_preview",
                &self.components.group_cache_chars,
                &self.project.engine,
                &self.project.store,
                self.project.canvas_w,
                self.project.canvas_h,
                s.common.no_syllables,
                if has_archaic_group {
                    Some(ui.available_height() / 2.0)
                } else {
                    None
                },
            );

            if has_archaic_group {
                ui.separator();
                ui.small(s.top.old_hangul);
                show_jamo_triplet_grid(
                    ui,
                    "comp_archaic_group",
                    &self.components.archaic_group_cache_triplets,
                    &self.project.engine,
                    &self.project.store,
                    self.project.canvas_w,
                    self.project.canvas_h,
                    s.common.no_syllables,
                );
            }
        }
    }
}

fn render_group_backrefs(ui: &mut egui::Ui, refs: &[GroupRef], s: &crate::i18n::Strings) {
    if refs.is_empty() {
        ui.weak(s.components.backref_none);
        return;
    }
    egui::Grid::new("group_backrefs_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for r in refs {
                ui.label(&r.template_name);
                let role = match &r.rule_name {
                    None => s.components.backref_default.to_string(),
                    Some(rn) => format!("{} \"{}\"", s.components.backref_variant, rn),
                };
                ui.label(role);
                let comp = match r.component {
                    HangulComponent::Cho => s.common.cho,
                    HangulComponent::Jung => s.common.jung,
                    HangulComponent::Jong => s.common.jong,
                };
                ui.label(comp);
                ui.end_row();
            }
        });
}

fn archaic_triplets_for_group(editor: &FontEditor, group_id: &str) -> Vec<JamoTriplet> {
    let Some(group) = editor.project.engine.rules.find_group(group_id) else {
        return vec![];
    };
    let target = group.target;
    let modern = allowed_chars_for_target(target);
    let modern_cho = allowed_chars_for_target(HangulComponent::Cho);
    let modern_jung = allowed_chars_for_target(HangulComponent::Jung);
    let ref_cho = modern_cho[0];
    let ref_jung = modern_jung[0];

    let archaic: Vec<char> = group.members.iter().copied().filter(|ch| !modern.contains(ch)).collect();

    let ref_jong = allowed_chars_for_target(HangulComponent::Jong)[0];

    let mut out = Vec::new();
    match target {
        HangulComponent::Cho => {
            for &c in &archaic {
                for &j in modern_jung {
                    if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, j, None)
                        && l.cho.group_id == group_id
                    {
                        out.push((c, j, None));
                    }
                    if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, j, Some(ref_jong))
                        && l.cho.group_id == group_id
                    {
                        out.push((c, j, Some(ref_jong)));
                    }
                }
            }
        }
        HangulComponent::Jung => {
            for &j in &archaic {
                for &c in modern_cho {
                    if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, j, None)
                        && l.jung.group_id == group_id
                    {
                        out.push((c, j, None));
                    }
                    if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, j, Some(ref_jong))
                        && l.jung.group_id == group_id
                    {
                        out.push((c, j, Some(ref_jong)));
                    }
                }
            }
        }
        HangulComponent::Jong => {
            for &k in &archaic {
                if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, ref_cho, ref_jung, Some(k))
                    && l.jong.as_ref().is_some_and(|p| p.group_id == group_id)
                {
                    out.push((ref_cho, ref_jung, Some(k)));
                }
            }
        }
    }
    out
}

/// used_chars_using_component의 옛한글 버전 — group_id가 다른 그룹 선택 시 다른 결과를 냄.
fn archaic_triplets_for_component_in_group(editor: &FontEditor, target: HangulComponent, jamo: char, group_id: &str) -> Vec<JamoTriplet> {
    let modern = allowed_chars_for_target(target);
    if modern.contains(&jamo) {
        return vec![];
    }
    let cho_list = allowed_chars_for_target(HangulComponent::Cho);
    let jung_list = allowed_chars_for_target(HangulComponent::Jung);
    let ref_jung = jung_list[0];
    let ref_jong = allowed_chars_for_target(HangulComponent::Jong)[0];

    let mut out = vec![];
    match target {
        HangulComponent::Cho => {
            for &j in jung_list {
                if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, jamo, j, None)
                    && l.cho.group_id == group_id
                {
                    out.push((jamo, j, None));
                }
                if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, jamo, j, Some(ref_jong))
                    && l.cho.group_id == group_id
                {
                    out.push((jamo, j, Some(ref_jong)));
                }
            }
        }
        HangulComponent::Jung => {
            for &c in cho_list {
                if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, jamo, None)
                    && l.jung.group_id == group_id
                {
                    out.push((c, jamo, None));
                }
            }
        }
        HangulComponent::Jong => {
            for &c in cho_list {
                if let Some(l) = editor.project.engine.layout_jamo(&editor.project.store, c, ref_jung, Some(jamo))
                    && l.jong.as_ref().is_some_and(|p| p.group_id == group_id)
                {
                    out.push((c, ref_jung, Some(jamo)));
                }
            }
        }
    }
    out
}

fn used_chars_using_group(editor: &FontEditor, group_id: &str) -> Vec<char> {
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
        let used = layout.cho.group_id == group_id
            || layout.jung.group_id == group_id
            || layout.jong.as_ref().is_some_and(|j| j.group_id == group_id);
        if used {
            out.push(ch);
        }
    }
    out
}

fn used_chars_using_component(editor: &FontEditor, target: HangulComponent, jamo: char, group_id: &str) -> Vec<char> {
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

        let used = match target {
            HangulComponent::Cho => layout.cho.placement.jamo == jamo && layout.cho.group_id == group_id,
            HangulComponent::Jung => layout.jung.placement.jamo == jamo && layout.jung.group_id == group_id,
            HangulComponent::Jong => layout
                .jong
                .as_ref()
                .is_some_and(|j| j.placement.jamo == jamo && j.group_id == group_id),
        };

        if used {
            out.push(ch);
        }
    }

    out
}
