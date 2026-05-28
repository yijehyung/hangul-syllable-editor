use std::collections::HashMap;

use eframe::egui;

use crate::app::{editor::FontEditor, ui_widgets::HangulCell};
use hangul_syllable::core::{
    hangul::{decompose_hangul, get_jamo_char},
    render::{ComposedPixel, compose_pixels},
};
use hangul_syllable::{ComponentGroup, HangulComponent};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum JongGroupFilter {
    #[default]
    Any,
    NoJong,
    Group(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct FilterKey {
    template_id: Option<String>,
    cho_group_id: Option<String>,
    jung_group_id: Option<String>,
    jong_group_id: JongGroupFilter,
    canvas_w: i32,
    canvas_h: i32,
}

#[derive(Clone, Debug)]
pub struct HangulBrowserState {
    pub search: String,

    pub template_id: Option<String>,
    pub cho_group_id: Option<String>,
    pub jung_group_id: Option<String>,
    pub jong_group_id: JongGroupFilter,

    pub cell_px: f32,

    pub jump_request_idx: Option<usize>,
    pub jump_highlight_idx: Option<usize>,

    pub pixel_cache: HashMap<char, Vec<ComposedPixel>>,
    pub cache_em: (i32, i32),

    pub filtered_list: Vec<char>,
    filter_key: FilterKey,
}

impl HangulBrowserState {
    pub fn reset_on_project_load(&mut self) {
        self.pixel_cache.clear();
        self.cache_em = (0, 0);
        self.filtered_list.clear();
    }
}

impl Default for HangulBrowserState {
    fn default() -> Self {
        Self {
            search: String::new(),
            template_id: None,
            cho_group_id: None,
            jung_group_id: None,
            jong_group_id: JongGroupFilter::Any,
            cell_px: 56.0,
            jump_request_idx: None,
            jump_highlight_idx: None,
            pixel_cache: HashMap::new(),
            cache_em: (0, 0),
            filtered_list: Vec::new(),
            filter_key: FilterKey::default(),
        }
    }
}

impl FontEditor {
    pub fn ui_hangul_browser_sidebar(&mut self, ui: &mut egui::Ui) {
        let s = crate::i18n::t(self.lang);
        ui.horizontal(|ui| {
            ui.heading(s.drawing.hangul_browser);
        });

        let mut jump_target_char: Option<char> = None;
        let mut jump_not_found = false;

        ui.horizontal(|ui| {
            ui.label(s.drawing.search_label);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.hangul_browser.search)
                    .hint_text(s.drawing.search_hint)
                    .desired_width(150.0),
            );

            let mut activate = false;
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                activate = true;
            }
            if ui.button("🔍").clicked() {
                activate = true;
            }

            if activate {
                jump_target_char = parse_hangul_query_to_char(&self.hangul_browser.search);
                if let Some(ch) = jump_target_char {
                    self.selected_char = ch;
                    self.target_text = ch.to_string();
                }
            }
        });

        ui.add_space(4.0);

        egui::Grid::new("hangul_filters").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            ui.label(s.common.template_label);
            let current = self
                .hangul_browser
                .template_id
                .as_deref()
                .and_then(|id| self.project.engine.rules.templates.iter().find(|t| t.id == id))
                .map(|t| if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() })
                .unwrap_or(s.widgets.any)
                .to_string();
            egui::ComboBox::from_id_salt("hangul_filter_template")
                .selected_text(current)
                .width(190.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.hangul_browser.template_id.is_none(), s.widgets.any)
                        .clicked()
                    {
                        self.hangul_browser.template_id = None;
                    }
                    for t in &self.project.engine.rules.templates {
                        let display = if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() };
                        let selected = self.hangul_browser.template_id.as_deref() == Some(&t.id);
                        if ui.selectable_label(selected, display).clicked() {
                            self.hangul_browser.template_id = Some(t.id.clone());
                        }
                    }
                });
            ui.end_row();

            ui.label(s.drawing.cho_grp);
            let mut cho_gid = self.hangul_browser.cho_group_id.clone();
            combo_group_filter(
                ui,
                HangulComponent::Cho,
                &self.project.engine.rules.groups,
                &mut cho_gid,
                "hf_cho",
                self.lang,
            );
            self.hangul_browser.cho_group_id = cho_gid;
            ui.end_row();

            ui.label(s.drawing.jung_grp);
            let mut jung_gid = self.hangul_browser.jung_group_id.clone();
            combo_group_filter(
                ui,
                HangulComponent::Jung,
                &self.project.engine.rules.groups,
                &mut jung_gid,
                "hf_jung",
                self.lang,
            );
            self.hangul_browser.jung_group_id = jung_gid;
            ui.end_row();

            ui.label(s.drawing.jong_grp);
            combo_jong_filter(
                ui,
                &self.project.engine.rules.groups,
                &mut self.hangul_browser.jong_group_id,
                "hf_jong",
                self.lang,
            );
            ui.end_row();
        });

        ui.add(egui::Slider::new(&mut self.hangul_browser.cell_px, 32.0..=88.0).text(s.drawing.cell_size));

        ui.separator();

        let list = self.collect_filtered_hangul_list_by_rules();

        if let Some(ch) = jump_target_char {
            if let Some(idx) = list.iter().position(|&x| x == ch) {
                self.hangul_browser.jump_request_idx = Some(idx);
            } else {
                jump_not_found = true;
            }
        }

        if jump_not_found {
            ui.colored_label(ui.visuals().warn_fg_color, s.drawing.filter_not_found);
        }

        ui.small(format!("Total: {}", list.len()));
        ui.add_space(4.0);

        self.ui_hangul_scrollable_grid(ui, &list);
    }

    pub fn collect_filtered_hangul_list_by_rules(&mut self) -> Vec<char> {
        let new_key = FilterKey {
            template_id: self.hangul_browser.template_id.clone(),
            cho_group_id: self.hangul_browser.cho_group_id.clone(),
            jung_group_id: self.hangul_browser.jung_group_id.clone(),
            jong_group_id: self.hangul_browser.jong_group_id.clone(),
            canvas_w: self.project.canvas_w,
            canvas_h: self.project.canvas_h,
        };

        if new_key == self.hangul_browser.filter_key && !self.hangul_browser.filtered_list.is_empty() {
            return self.hangul_browser.filtered_list.clone();
        }

        self.hangul_browser.filter_key = new_key;
        self.hangul_browser.filtered_list = self.build_filtered_hangul_list();
        self.hangul_browser.filtered_list.clone()
    }

    fn build_filtered_hangul_list(&self) -> Vec<char> {
        let any_template = self.hangul_browser.template_id.is_none();
        let any_cho = self.hangul_browser.cho_group_id.is_none();
        let any_jung = self.hangul_browser.jung_group_id.is_none();

        if any_template && any_cho && any_jung && self.hangul_browser.jong_group_id == JongGroupFilter::NoJong {
            let mut out = Vec::with_capacity(11172 / 28 + 10);
            for cp in 0xAC00u32..=0xD7A3u32 {
                let s = cp - 0xAC00;
                if s % 28 == 0
                    && let Some(ch) = char::from_u32(cp)
                {
                    out.push(ch);
                }
            }
            return out;
        }

        let mut out = Vec::with_capacity(11172);

        for cp in 0xAC00u32..=0xD7A3u32 {
            let Some(ch) = char::from_u32(cp) else {
                continue;
            };

            let Some(res) = self.project.engine.layout_char(
                &self.project.store,
                ch,
                hangul_syllable::core::hangul::decompose_hangul,
                hangul_syllable::core::hangul::get_jamo_char,
            ) else {
                continue;
            };

            if let Some(tid) = &self.hangul_browser.template_id
                && &res.template_id != tid
            {
                continue;
            }

            if let Some(gid) = &self.hangul_browser.cho_group_id
                && res.cho.group_id != *gid
            {
                continue;
            }

            if let Some(gid) = &self.hangul_browser.jung_group_id
                && res.jung.group_id != *gid
            {
                continue;
            }

            match &self.hangul_browser.jong_group_id {
                JongGroupFilter::Any => {}
                JongGroupFilter::NoJong => {
                    if (cp - 0xAC00) % 28 != 0 {
                        continue;
                    }
                }
                JongGroupFilter::Group(gid) => {
                    if res.jong.as_ref().map(|j| j.group_id.as_str()) != Some(gid.as_str()) {
                        continue;
                    }
                }
            }

            out.push(ch);
        }

        out
    }

    fn ui_hangul_scrollable_grid(&mut self, ui: &mut egui::Ui, list: &[char]) {
        let s = crate::i18n::t(self.lang);
        if self.hangul_browser.cache_em != (self.project.canvas_w, self.project.canvas_h) {
            self.hangul_browser.pixel_cache.clear();
            self.hangul_browser.cache_em = (self.project.canvas_w, self.project.canvas_h);
        }

        let spacing = 6.0;
        let cell = self.hangul_browser.cell_px;

        if list.is_empty() {
            ui.label(s.drawing.search_no_results);
            return;
        }

        ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
        let available_w = ui.available_width().max(1.0);
        let cols = (((available_w + spacing) / (cell + spacing)).floor() as usize).max(1);
        let total_count = list.len();
        let total_rows = total_count.div_ceil(cols);

        let row_h = cell;
        let row_advance = row_h + spacing;

        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("hangul_scroll_grid")
            .auto_shrink([false, false]);

        if let Some(idx) = self.hangul_browser.jump_request_idx.take() {
            self.hangul_browser.jump_highlight_idx = Some(idx);

            let target_row = idx / cols;
            let target_offset = target_row as f32 * row_advance;

            scroll = scroll.vertical_scroll_offset(target_offset);
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
                        self.draw_hangul_cell_in_rect(ui, ch);
                    }
                });
            }
        });
    }

    fn draw_hangul_cell_in_rect(&mut self, ui: &mut egui::Ui, ch: char) {
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
            self.hangul_browser.cell_px,
            is_selected,
        ));

        if resp.clicked() {
            self.selected_char = ch;
            self.target_text = ch.to_string();
        }
        if resp.hovered() {
            resp.on_hover_text(format!("'{}'  U+{:04X}", ch, ch as u32));
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
    }
}

pub fn parse_hangul_query_to_char(q: &str) -> Option<char> {
    let s = q.trim();
    if s.is_empty() {
        return None;
    }

    if s.chars().count() == 1
        && let Some(ch) = s.chars().next()
        && (0xAC00..=0xD7A3).contains(&(ch as u32))
    {
        return Some(ch);
    }

    let upper = s.to_uppercase();
    let hex = upper.strip_prefix("U+").unwrap_or(&upper);

    if (2..=6).contains(&hex.len())
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && let Ok(cp) = u32::from_str_radix(hex, 16)
        && (0xAC00..=0xD7A3).contains(&cp)
    {
        return char::from_u32(cp);
    }

    None
}

pub fn combo_group_filter(
    ui: &mut egui::Ui,
    target: HangulComponent,
    groups: &[ComponentGroup],
    selected_gid: &mut Option<String>,
    id_salt: &str,
    lang: crate::i18n::Lang,
) {
    let w = crate::i18n::t(lang);

    let selected_text = match selected_gid.as_deref() {
        None => w.widgets.any.to_string(),
        Some(gid) => groups
            .iter()
            .find(|g| g.id == gid)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    };

    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .width(190.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected_gid, None, w.widgets.any);
            for g in groups.iter().filter(|g| g.target == target) {
                ui.selectable_value(selected_gid, Some(g.id.clone()), g.name.as_str());
            }
        });
}

pub fn combo_jong_filter(
    ui: &mut egui::Ui,
    groups: &[ComponentGroup],
    selected: &mut JongGroupFilter,
    id_salt: &str,
    lang: crate::i18n::Lang,
) {
    let w = crate::i18n::t(lang);

    let selected_text = match selected {
        JongGroupFilter::Any => w.widgets.any.to_string(),
        JongGroupFilter::NoJong => w.drawing.no_jong.to_string(),
        JongGroupFilter::Group(gid) => groups
            .iter()
            .find(|g| g.target == HangulComponent::Jong && g.id == *gid)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    };

    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .width(190.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, JongGroupFilter::Any, w.widgets.any);
            ui.selectable_value(selected, JongGroupFilter::NoJong, w.drawing.no_jong);
            for g in groups.iter().filter(|g| g.target == HangulComponent::Jong) {
                ui.selectable_value(selected, JongGroupFilter::Group(g.id.clone()), g.name.as_str());
            }
        });
}
