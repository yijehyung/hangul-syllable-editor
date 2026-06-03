use std::collections::BTreeSet;

use eframe::egui;

use crate::app::ui_drawing::cell_colors as cc;
use hangul_syllable::core::{
    groups::group_display_name,
    hangul::{allowed_chars_for_target, decompose_hangul, get_jamo_char},
    render::{ComposedPixel, compose_pixels},
};
use hangul_syllable::{CharSetCond, ComponentGroup, GlyphStore, HangulComponent, LayoutEngine};

/// 옛한글용: (cho, jung, jong_opt) 트리플렛을 layout_jamo로 렌더링
pub type JamoTriplet = (char, char, Option<char>);

pub(super) fn move_vec<T>(v: &mut Vec<T>, from: usize, to: usize) {
    if v.is_empty() || from >= v.len() {
        return;
    }
    let item = v.remove(from);
    let to = to.min(v.len());
    v.insert(to, item);
}

pub struct HangulCell<'a> {
    ch: char,
    pixels: Option<&'a [ComposedPixel]>,
    canvas_w: i32,
    canvas_h: i32,
    cell_px: f32,
    is_selected: bool,
}

impl<'a> HangulCell<'a> {
    pub fn new(ch: char, pixels: Option<&'a [ComposedPixel]>, canvas_w: i32, canvas_h: i32, cell_px: f32, is_selected: bool) -> Self {
        Self {
            ch,
            pixels,
            canvas_w,
            canvas_h,
            cell_px,
            is_selected,
        }
    }
}

impl egui::Widget for HangulCell<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let cell_size = egui::vec2(self.cell_px, self.cell_px);
        let (rect, resp) = ui.allocate_exact_size(cell_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let hovered = resp.hovered();
            let dark = ui.visuals().dark_mode;
            let painter = ui.painter();

            let bg = if self.is_selected {
                cc::bg_selected(dark)
            } else if hovered {
                cc::bg_hovered(dark)
            } else {
                cc::bg_normal(dark)
            };
            painter.rect_filled(rect, 2.0, bg);
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(
                    1.0,
                    if self.is_selected {
                        cc::border_selected(dark)
                    } else {
                        cc::border_normal(dark)
                    },
                ),
                egui::StrokeKind::Middle,
            );

            let pad = 4.0;
            let label_h = 14.0;
            painter.text(
                egui::pos2(rect.center().x, rect.min.y + pad),
                egui::Align2::CENTER_TOP,
                self.ch.to_string(),
                egui::FontId::proportional(12.0),
                ui.visuals().text_color(),
            );

            let preview_rect = egui::Rect::from_min_max(rect.min + egui::vec2(pad, pad + label_h), rect.max - egui::vec2(pad, pad));

            if let Some(pixels) = self.pixels {
                let canvas_w = self.canvas_w.max(1) as f32;
                let canvas_h = self.canvas_h.max(1) as f32;
                let zoom = (preview_rect.width() / canvas_w).min(preview_rect.height() / canvas_h).max(1.0);
                let origin = egui::pos2(
                    preview_rect.center().x - canvas_w * zoom * 0.5,
                    preview_rect.center().y - canvas_h * zoom * 0.5,
                );
                let ink = if ui.visuals().dark_mode {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::BLACK
                };

                for px in pixels {
                    let px_rect = egui::Rect::from_min_size(
                        egui::pos2(origin.x + px.x as f32 * zoom, origin.y + px.y as f32 * zoom),
                        egui::vec2(zoom, zoom),
                    );
                    if preview_rect.intersects(px_rect) {
                        painter.rect_filled(px_rect, 0.0, ink);
                    }
                }
            }
        }

        resp
    }
}

pub fn pick_group_combo(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    target: HangulComponent,
    groups: &[ComponentGroup],
    selected_group_id: &mut String,
) {
    let list: Vec<&ComponentGroup> = groups.iter().filter(|g| g.target == target).collect();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("{label}:"));
        let current = group_display_name(groups, selected_group_id);

        let width = ui.available_width().clamp(96.0, 260.0);
        egui::ComboBox::from_id_salt(id)
            .selected_text(current)
            .width(width)
            .show_ui(ui, |ui| {
                for g in list {
                    ui.selectable_value(selected_group_id, g.id.clone(), g.name.clone());
                }
            });
    });
}

pub fn ui_charset_toggle(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    cond: &mut CharSetCond,
    allowed: &[char],
    _per_row: usize,
    lang: crate::i18n::Lang,
) {
    let base_id = id;
    let w = crate::i18n::t(lang);

    let show_none_for_zero = allowed.contains(&'\0');

    let disp = |ch: char| -> String {
        if show_none_for_zero && ch == '\0' {
            w.widgets.none_label.to_string()
        } else {
            ch.to_string()
        }
    };

    let max_width = (visible_width(ui) - 12.0).max(1.0);
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_width(max_width);
        ui.set_max_width(max_width);
        ui.label(label);

        let mode_text = match cond {
            CharSetCond::Any => w.widgets.any,
            CharSetCond::Include(_) => w.widgets.include,
            CharSetCond::Exclude(_) => w.widgets.exclude,
        };

        let mut set_mode: Option<u8> = None;

        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt(base_id.with("mode_combo"))
                .selected_text(mode_text)
                .width(ui.available_width().clamp(84.0, 140.0))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(matches!(cond, CharSetCond::Any), w.widgets.any).clicked() {
                        set_mode = Some(0);
                    }
                    if ui
                        .selectable_label(matches!(cond, CharSetCond::Include(_)), w.widgets.include)
                        .clicked()
                    {
                        set_mode = Some(1);
                    }
                    if ui
                        .selectable_label(matches!(cond, CharSetCond::Exclude(_)), w.widgets.exclude)
                        .clicked()
                    {
                        set_mode = Some(2);
                    }
                });

            if ui.small_button(w.widgets.clear).clicked() {
                cond.clear();
            }
        });

        if let Some(m) = set_mode {
            *cond = match m {
                0 => CharSetCond::Any,
                1 => CharSetCond::Include(BTreeSet::new()),
                _ => CharSetCond::Exclude(BTreeSet::new()),
            };
        }

        let enabled = !matches!(cond, CharSetCond::Any);

        ui.add_enabled_ui(enabled, |ui| {
            let cell_w = 46.0;
            let gap = 4.0;
            let cols = (((max_width + gap) / (cell_w + gap)).floor() as usize).clamp(1, 10);
            for row in allowed.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &ch in row {
                        let on = match cond {
                            CharSetCond::Include(set) | CharSetCond::Exclude(set) => set.contains(&ch),
                            CharSetCond::Any => false,
                        };

                        if ui
                            .add_sized(egui::vec2(cell_w, 24.0), egui::Button::new(disp(ch)).selected(on))
                            .clicked()
                        {
                            cond.toggle(ch);
                        }
                    }
                });
            }
        });

        if let CharSetCond::Include(set) | CharSetCond::Exclude(set) = cond
            && !set.is_empty()
        {
            ui.horizontal_wrapped(|ui| {
                ui.small(w.widgets.selected_prefix);
                let mut v: Vec<char> = set.iter().copied().collect();

                v.sort_by_key(|c| if *c == '\0' { 0u8 } else { 1u8 });

                for ch in v {
                    ui.small(disp(ch));
                }
            });
        }
    });
}

pub fn sub_panel_tabs(ui: &mut egui::Ui, selected: &mut usize, labels: &[&str]) {
    ui.horizontal_wrapped(|ui| {
        for (i, &label) in labels.iter().enumerate() {
            if ui.selectable_label(*selected == i, label).clicked() {
                *selected = i;
            }
        }
    });
}

pub fn columns_for_width(ui: &egui::Ui, cell_width: f32, gap: f32, max_cols: usize) -> usize {
    let width = visible_width(ui).max(cell_width);
    let cols = ((width + gap) / (cell_width + gap)).floor() as usize;
    cols.clamp(1, max_cols.max(1))
}

pub fn visible_width(ui: &egui::Ui) -> f32 {
    ui.available_width().min(ui.max_rect().width()).min(ui.clip_rect().width()).max(1.0)
}

pub fn ui_separator_soft(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}

pub fn draw_glyph_thumbnail(
    ui: &mut egui::Ui,
    store: &hangul_syllable::core::glyph::GlyphStore,
    key: &hangul_syllable::core::glyph::GlyphKey,
    size: f32,
    canvas_w: i32,
    canvas_h: i32,
) -> egui::Response {
    let (resp, painter) = ui.allocate_painter(egui::vec2(size, size), egui::Sense::click());
    let r = resp.rect;

    let dark = ui.visuals().dark_mode;
    painter.rect_filled(
        r,
        3.0,
        if dark {
            egui::Color32::from_gray(10)
        } else {
            egui::Color32::from_gray(240)
        },
    );
    painter.rect_stroke(
        r,
        3.0,
        egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_gray(40)
            } else {
                egui::Color32::from_gray(160)
            },
        ),
        egui::StrokeKind::Middle,
    );

    if let Some(g) = store.get(key) {
        let dim = canvas_w.max(canvas_h).max(1) as f32;
        let scale = (size / dim).max(0.5);
        let ink = if ui.visuals().dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::BLACK
        };
        for &(x, y) in &g.pixels {
            if x < 0 || y < 0 || x >= canvas_w || y >= canvas_h {
                continue;
            }
            let px = r.min.x + x as f32 * scale;
            let py = r.min.y + y as f32 * scale;
            let pr = egui::Rect::from_min_size(egui::pos2(px, py), egui::vec2(scale, scale));
            painter.rect_filled(pr, 0.0, ink);
        }
    }

    resp
}

/// 유효(effective) 음절은 기본 색, 덮어씌워진(overridden) 음절은 주황색으로 표시.
#[allow(clippy::too_many_arguments)]
pub fn show_syllable_grid_two_tone(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    effective: &[char],
    overridden: &[char],
    engine: &LayoutEngine,
    store: &GlyphStore,
    canvas_w: i32,
    canvas_h: i32,
    no_syllables_text: &str,
) {
    if effective.is_empty() && overridden.is_empty() {
        ui.weak(no_syllables_text);
        return;
    }

    let overridden_set: std::collections::HashSet<char> = overridden.iter().copied().collect();
    let mut all: Vec<char> = effective.iter().chain(overridden.iter()).copied().collect();
    all.sort_unstable();

    const THUMB: f32 = 44.0;
    const GAP: f32 = 4.0;
    const ROW_H: f32 = THUMB + 18.0;

    let cols = ((ui.available_width() + GAP) / (THUMB + GAP)).floor().clamp(1.0, 12.0) as usize;
    let total_rows = all.len().div_ceil(cols);
    let dim = canvas_w.max(canvas_h).max(1) as f32;
    let zoom = (THUMB / dim).max(0.5);

    let ink_normal = if ui.visuals().dark_mode {
        egui::Color32::WHITE
    } else {
        egui::Color32::BLACK
    };
    let ink_dim = egui::Color32::from_rgb(200, 110, 40);

    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show_rows(ui, ROW_H, total_rows, |ui, row_range| {
            for row in row_range {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let i = row * cols + col;
                        if i >= all.len() {
                            break;
                        }
                        let ch = all[i];
                        let is_overridden = overridden_set.contains(&ch);
                        let ink = if is_overridden { ink_dim } else { ink_normal };

                        ui.vertical(|ui| {
                            ui.set_min_width(THUMB);
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(THUMB, THUMB), egui::Sense::hover());

                            if let Some(layout) = engine.layout_char(store, ch, decompose_hangul, get_jamo_char) {
                                let pixels = compose_pixels(store, &layout, canvas_w, canvas_h);
                                let painter = ui.painter_at(rect);
                                let ox = rect.center().x - canvas_w as f32 * zoom * 0.5;
                                let oy = rect.center().y - canvas_h as f32 * zoom * 0.5;
                                for px in &pixels {
                                    let pr = egui::Rect::from_min_size(
                                        egui::pos2(ox + px.x as f32 * zoom, oy + px.y as f32 * zoom),
                                        egui::vec2(zoom, zoom),
                                    );
                                    painter.rect_filled(pr, 0.0, ink);
                                }
                            }

                            ui.label(egui::RichText::new(ch.to_string()).small().color(ink));
                        });
                    }
                });
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn show_syllable_grid(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    sylls: &[char],
    engine: &LayoutEngine,
    store: &GlyphStore,
    canvas_w: i32,
    canvas_h: i32,
    no_syllables_text: &str,
    max_height: Option<f32>,
) {
    if sylls.is_empty() {
        ui.weak(no_syllables_text);
        return;
    }

    const THUMB: f32 = 44.0;
    const GAP: f32 = 4.0;
    const ROW_H: f32 = THUMB + 18.0;

    let cols = ((ui.available_width() + GAP) / (THUMB + GAP)).floor().clamp(1.0, 12.0) as usize;
    let total_rows = sylls.len().div_ceil(cols);
    let dim = canvas_w.max(canvas_h).max(1) as f32;
    let zoom = (THUMB / dim).max(0.5);

    let mut scroll = egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]);
    if let Some(h) = max_height {
        scroll = scroll.max_height(h);
    }
    scroll.show_rows(ui, ROW_H, total_rows, |ui, row_range| {
        for row in row_range {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let i = row * cols + col;
                    if i >= sylls.len() {
                        break;
                    }
                    let ch = sylls[i];

                    ui.vertical(|ui| {
                        ui.set_min_width(THUMB);
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(THUMB, THUMB), egui::Sense::hover());

                        if let Some(layout) = engine.layout_char(store, ch, decompose_hangul, get_jamo_char) {
                            let pixels = compose_pixels(store, &layout, canvas_w, canvas_h);
                            let ink = if ui.visuals().dark_mode {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::BLACK
                            };
                            let painter = ui.painter_at(rect);
                            let ox = rect.center().x - canvas_w as f32 * zoom * 0.5;
                            let oy = rect.center().y - canvas_h as f32 * zoom * 0.5;
                            for px in &pixels {
                                let pr = egui::Rect::from_min_size(
                                    egui::pos2(ox + px.x as f32 * zoom, oy + px.y as f32 * zoom),
                                    egui::vec2(zoom, zoom),
                                );
                                painter.rect_filled(pr, 0.0, ink);
                            }
                        }

                        ui.small(ch.to_string());
                    });
                }
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn show_jamo_triplet_grid(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    entries: &[JamoTriplet],
    engine: &LayoutEngine,
    store: &GlyphStore,
    canvas_w: i32,
    canvas_h: i32,
    no_syllables_text: &str,
) {
    if entries.is_empty() {
        ui.weak(no_syllables_text);
        return;
    }

    const THUMB: f32 = 44.0;
    const GAP: f32 = 4.0;
    const ROW_H: f32 = THUMB + 18.0;

    let cols = ((ui.available_width() + GAP) / (THUMB + GAP)).floor().clamp(1.0, 12.0) as usize;
    let total_rows = entries.len().div_ceil(cols);
    let dim = canvas_w.max(canvas_h).max(1) as f32;
    let zoom = (THUMB / dim).max(0.5);

    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show_rows(ui, ROW_H, total_rows, |ui, row_range| {
            for row in row_range {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let i = row * cols + col;
                        if i >= entries.len() {
                            break;
                        }
                        let (cho, jung, jong) = entries[i];
                        let label = match jong {
                            None => format!("{}{}", to_initial_jamo(cho), to_medial_jamo(jung)),
                            Some(j) => format!("{}{}{}", to_initial_jamo(cho), to_medial_jamo(jung), to_final_jamo(j)),
                        };

                        ui.vertical(|ui| {
                            ui.set_min_width(THUMB);
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(THUMB, THUMB), egui::Sense::hover());

                            if let Some(layout) = engine.layout_jamo(store, cho, jung, jong) {
                                let pixels = compose_pixels(store, &layout, canvas_w, canvas_h);
                                let ink = if ui.visuals().dark_mode {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::BLACK
                                };
                                let painter = ui.painter_at(rect);
                                let ox = rect.center().x - canvas_w as f32 * zoom * 0.5;
                                let oy = rect.center().y - canvas_h as f32 * zoom * 0.5;
                                for px in &pixels {
                                    let pr = egui::Rect::from_min_size(
                                        egui::pos2(ox + px.x as f32 * zoom, oy + px.y as f32 * zoom),
                                        egui::vec2(zoom, zoom),
                                    );
                                    painter.rect_filled(pr, 0.0, ink);
                                }
                            }

                            ui.small(label);
                        });
                    }
                });
            }
        });
}

fn to_initial_jamo(ch: char) -> char {
    if let Some(i) = allowed_chars_for_target(HangulComponent::Cho).iter().position(|&c| c == ch) {
        char::from_u32(0x1100 + i as u32).unwrap_or(ch)
    } else {
        ch
    }
}

fn to_medial_jamo(ch: char) -> char {
    if let Some(i) = allowed_chars_for_target(HangulComponent::Jung).iter().position(|&c| c == ch) {
        char::from_u32(0x1161 + i as u32).unwrap_or(ch)
    } else {
        ch
    }
}

fn to_final_jamo(ch: char) -> char {
    if let Some(i) = allowed_chars_for_target(HangulComponent::Jong).iter().position(|&c| c == ch) {
        char::from_u32(0x11A8 + i as u32).unwrap_or(ch)
    } else {
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_vec_forward() {
        let mut v = vec![1, 2, 3, 4, 5];
        move_vec(&mut v, 0, 3);
        assert_eq!(v, vec![2, 3, 4, 1, 5]);
    }

    #[test]
    fn move_vec_backward() {
        let mut v = vec![1, 2, 3, 4, 5];
        move_vec(&mut v, 4, 1);
        assert_eq!(v, vec![1, 5, 2, 3, 4]);
    }

    #[test]
    fn move_vec_same_position() {
        let mut v = vec![1, 2, 3];
        move_vec(&mut v, 1, 1);
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn move_vec_out_of_bounds_from() {
        let mut v = vec![1, 2, 3];
        move_vec(&mut v, 5, 0);
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn move_vec_to_clamps_to_end() {
        let mut v = vec![1, 2, 3];
        move_vec(&mut v, 0, 100);
        assert_eq!(v, vec![2, 3, 1]);
    }

    #[test]
    fn move_vec_empty() {
        let mut v: Vec<i32> = vec![];
        move_vec(&mut v, 0, 0);
        assert_eq!(v, vec![]);
    }
}
