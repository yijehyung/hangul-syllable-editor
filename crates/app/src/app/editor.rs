use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use eframe::egui;
use hangul_syllable::core::{
    generator::RuleGenerator,
    hangul::{decompose_hangul, get_jamo_char},
    render::{ComposedPixel, RenderContext},
};
use hangul_syllable::io::export::{CharScope, ExportConfig, FileNameFormat, build_sheet_image, get_char_list};
use hangul_syllable::{
    GlyphKey, GlyphStore, HangulComponent, LayoutEngine, LayoutResult, PixelGlyph, ProjectData, apply_old_hangul_rules,
    default_archaic_map, parse_project_bytes,
};

use crate::app::deferred::Deferred;
use crate::app::ui_hangul_browser::HangulBrowserState;

pub const NARROW_WIDTH: f32 = 768.0;

type OpenPromise = Option<Deferred<Option<(Vec<u8>, Option<PathBuf>)>>>;
type RgbaImageVec = Vec<(char, image::ImageBuffer<image::Rgba<u8>, Vec<u8>>)>;

#[derive(Clone, Debug)]
pub enum DiscardAction {
    NewDefault,
    NewMinzkn,
    NewZik,
    NewDkb,
    NewHanterm,
    OpenFile,
}

#[derive(PartialEq)]
pub enum EditorMode {
    Drawing,
    Components,
    Templates,
    SelectorRules,
    Syllables,
    TypeTest,
    OldHangulMap,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    Draw,
    Erase,
}

pub struct ExportState {
    pub show_window: bool,
    pub target_type: CharScope,
    pub custom_text: String,
    pub sheet_columns: u32,
    pub text_color: [f32; 3],
    pub bg_color: [f32; 3],
    pub bg_transparent: bool,
    pub name_format: FileNameFormat,
    pub sheet_promise: Option<Deferred<Option<()>>>,
    pub individual_promise: Option<Deferred<Option<()>>>,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            show_window: false,
            target_type: CharScope::default(),
            custom_text: String::new(),
            sheet_columns: 32,
            text_color: [1.0, 1.0, 1.0],
            bg_color: [0.0, 0.0, 0.0],
            bg_transparent: true,
            name_format: FileNameFormat::default(),
            sheet_promise: None,
            individual_promise: None,
        }
    }
}

pub struct TypeTestState {
    pub text: String,
    pub zoom: f32,
    pub letter_spacing: f32,
    pub line_spacing: f32,
    pub font_size: f32,
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub shadow_enabled: bool,
    pub shadow_color: egui::Color32,
    pub shadow_px: i32,
    pub shadow_dirs: [bool; 8],
    pub space_px: i32,
    pub pixel_cache: HashMap<char, Vec<ComposedPixel>>,
    pub triplet_cache: HashMap<(char, char, Option<char>), Vec<ComposedPixel>>,
    pub cache_em: (i32, i32),
}

impl Default for TypeTestState {
    fn default() -> Self {
        Self {
            text: "다람쥐 헌 쳇바퀴에 타고파\n키스의 고유 조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다".into(),
            zoom: 5.0,
            letter_spacing: 0.0,
            line_spacing: 6.0,
            font_size: 24.0,
            bg: egui::Color32::from_gray(15),
            fg: egui::Color32::WHITE,
            shadow_enabled: false,
            shadow_color: egui::Color32::from_gray(55),
            shadow_px: 1,
            shadow_dirs: [false, false, true, true, true, false, false, false],
            space_px: 4,
            pixel_cache: HashMap::new(),
            triplet_cache: HashMap::new(),
            cache_em: (0, 0),
        }
    }
}

impl TypeTestState {
    pub fn reset_on_project_load(&mut self) {
        self.pixel_cache.clear();
        self.triplet_cache.clear();
        self.cache_em = (0, 0);
    }
}

pub struct DrawingState {
    pub grid_size: f32,
    pub paint_mode: Option<PaintMode>,
    pub last_paint_cell: Option<(i32, i32)>,
    pub preview_pixels: Vec<ComposedPixel>,
    pub preview_key: (char, i32, i32),
    pub preview_dirty: bool,
    pub selected_part_tab: u8,
    pub undo_stack: Vec<(GlyphKey, BTreeSet<(i32, i32)>)>,
    pub redo_stack: Vec<(GlyphKey, BTreeSet<(i32, i32)>)>,
    pub(super) stroke_start: Option<(GlyphKey, BTreeSet<(i32, i32)>)>,
    pub pixel_clipboard: Option<BTreeSet<(i32, i32)>>,
    pub active_edit_key: Option<GlyphKey>,
    pub pen_toggle_mode: bool,
}

impl Default for DrawingState {
    fn default() -> Self {
        Self {
            grid_size: 24.0,
            paint_mode: None,
            last_paint_cell: None,
            preview_pixels: Vec::new(),
            preview_key: ('\0', 0, 0),
            preview_dirty: true,
            selected_part_tab: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            stroke_start: None,
            pixel_clipboard: None,
            active_edit_key: None,
            pen_toggle_mode: false,
        }
    }
}

#[derive(Default)]
pub struct ComponentsState {
    pub selected_group: usize,
    pub selected_member_index: Option<char>,
    pub group_filter: Option<HangulComponent>,
    pub usage_cache_key: Option<(String, HangulComponent, char)>,
    pub usage_cache_chars: Vec<char>,
    pub group_cache_key: Option<String>,
    pub group_cache_chars: Vec<char>,
    pub archaic_cache_key: Option<(String, HangulComponent, char)>,
    pub archaic_cache_triplets: Vec<(char, char, Option<char>)>,
    pub archaic_group_cache_key: Option<String>,
    pub archaic_group_cache_triplets: Vec<(char, char, Option<char>)>,
}

impl ComponentsState {
    pub fn reset_on_project_load(&mut self) {
        self.selected_group = 0;
        self.selected_member_index = None;
        self.usage_cache_key = None;
        self.group_cache_key = None;
        self.archaic_cache_key = None;
        self.archaic_group_cache_key = None;
    }
}

#[derive(Default)]
pub struct TemplateEditorState {
    pub selected_template: usize,
    pub usage_cache_key: Option<String>,
    pub usage_cache_chars: Vec<char>,
    pub usage_cache_triplets: Vec<(char, char, Option<char>)>,
    pub selected_variant_rule: Option<String>,
    pub variant_cache_key: Option<(String, String)>,
    pub variant_cache_chars: Vec<char>,
    pub variant_cache_triplets: Vec<(char, char, Option<char>)>,
}

impl TemplateEditorState {
    pub fn reset_on_project_load(&mut self) {
        self.selected_template = 0;
        self.usage_cache_key = None;
        self.usage_cache_chars.clear();
        self.usage_cache_triplets.clear();
        self.selected_variant_rule = None;
        self.variant_cache_key = None;
        self.variant_cache_chars.clear();
        self.variant_cache_triplets.clear();
    }
}

#[derive(Default)]
pub struct SelEditorState {
    pub cache_key: Option<String>,
    pub effective_chars: Vec<char>,
    pub overridden_chars: Vec<char>,
}

impl SelEditorState {
    pub fn reset_on_project_load(&mut self) {
        self.cache_key = None;
        self.effective_chars.clear();
        self.overridden_chars.clear();
    }
}

pub struct NewGroupDraft {
    pub name: String,
    pub target: HangulComponent,
}

impl Default for NewGroupDraft {
    fn default() -> Self {
        Self {
            name: "".into(),
            target: HangulComponent::Cho,
        }
    }
}

#[derive(Clone)]
pub struct AppNotification {
    pub message: String,
    pub is_error: bool,
    expire_at: Option<f64>,
}

impl AppNotification {
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            is_error: true,
            expire_at: None,
        }
    }
}

pub struct ProjectState {
    pub store: GlyphStore,
    pub engine: LayoutEngine,
    pub canvas_w: i32,
    pub canvas_h: i32,
    pub current_path: Option<std::path::PathBuf>,
    pub is_dirty: bool,
    pub old_hangul_enabled: bool,
    pub archaic_jamo_map: Vec<(char, char)>,
}

impl Default for ProjectState {
    fn default() -> Self {
        let mut store = GlyphStore::default();
        let engine = LayoutEngine {
            rules: RuleGenerator::generate_default(&mut store),
        };
        Self {
            store,
            engine,
            canvas_w: 12,
            canvas_h: 12,
            current_path: None,
            is_dirty: false,
            old_hangul_enabled: false,
            archaic_jamo_map: default_archaic_map(),
        }
    }
}

impl ProjectState {
    pub fn load_data(&mut self, data: ProjectData) {
        self.canvas_w = data.canvas_w;
        self.canvas_h = data.canvas_h;
        self.store = data.store;
        self.engine.rules = data.rules;
        self.old_hangul_enabled = data.old_hangul_enabled;
        self.archaic_jamo_map = data.archaic_jamo_map;
        self.is_dirty = false;
    }

    pub fn to_project_data(&self) -> ProjectData {
        ProjectData::from_editor(
            self.canvas_w,
            self.canvas_h,
            &self.store,
            &self.engine.rules,
            self.old_hangul_enabled,
            self.archaic_jamo_map.clone(),
        )
    }
}

pub struct FontEditor {
    pub project: ProjectState,

    pub mode: EditorMode,

    pub target_text: String,
    pub selected_char: char,

    pub drawing: DrawingState,

    pub selected_selector: usize,

    pub sel_editor: SelEditorState,

    pub tpl_editor: TemplateEditorState,

    pub components: ComponentsState,

    pub new_group: NewGroupDraft,

    pub hangul_browser: HangulBrowserState,

    pub export_state: ExportState,

    pub type_test: TypeTestState,

    pub lang: crate::i18n::Lang,

    pub open_promise: OpenPromise,
    pub save_as_promise: Option<Deferred<Option<PathBuf>>>,

    pub show_close_dialog: bool,
    pub allowed_to_close: bool,
    pub show_discard_dialog: bool,
    pub pending_discard: Option<DiscardAction>,

    pub show_apply_old_hangul_dialog: bool,
    pub apply_old_hangul_copy_pixels: bool,
    pub old_hangul_filter: Option<HangulComponent>,

    pub notifications: Vec<AppNotification>,

    pub narrow_draw_sub: usize,
    pub narrow_comp_sub: usize,
    pub narrow_tpl_sub: usize,
    pub narrow_sel_sub: usize,
    pub narrow_tt_sub: usize,

    last_title: String,
}

impl Default for FontEditor {
    fn default() -> Self {
        Self {
            project: ProjectState::default(),

            mode: EditorMode::Drawing,
            target_text: "가".to_string(),
            selected_char: '가',
            drawing: DrawingState::default(),
            selected_selector: 0,
            sel_editor: SelEditorState::default(),
            tpl_editor: TemplateEditorState::default(),
            components: ComponentsState::default(),
            new_group: NewGroupDraft::default(),
            hangul_browser: HangulBrowserState::default(),
            export_state: ExportState::default(),
            type_test: TypeTestState::default(),
            lang: crate::i18n::Lang::default(),
            open_promise: None,
            save_as_promise: None,
            show_close_dialog: false,
            allowed_to_close: false,
            show_discard_dialog: false,
            pending_discard: None,
            show_apply_old_hangul_dialog: false,
            apply_old_hangul_copy_pixels: false,
            old_hangul_filter: None,
            notifications: Vec::new(),

            narrow_draw_sub: 0,
            narrow_comp_sub: 0,
            narrow_tpl_sub: 0,
            narrow_sel_sub: 0,
            narrow_tt_sub: 0,

            last_title: String::new(),
        }
    }
}

impl eframe::App for FontEditor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        {
            let file_name = self
                .project
                .current_path
                .as_deref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or(crate::i18n::t(self.lang).top.new_file);
            let prefix = if self.project.is_dirty { "*" } else { "" };
            let title = format!("{prefix}{file_name} — Hangul Syllable Editor");
            if title != self.last_title {
                self.last_title = title.clone();
                #[cfg(not(target_arch = "wasm32"))]
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                #[cfg(target_arch = "wasm32")]
                crate::platform::set_page_title(&title);
            }
            #[cfg(target_arch = "wasm32")]
            crate::platform::set_unload_guard(self.project.is_dirty);
        }

        self.poll_promises(&ctx);

        {
            let undo = ctx.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift);
            let redo = ctx.input(|i| {
                (i.key_pressed(egui::Key::Y) && i.modifiers.ctrl) || (i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && i.modifiers.shift)
            });
            let save = ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl);
            if undo {
                self.undo();
            }
            if redo {
                self.redo();
            }
            if save {
                self.save_project_now();
            }
        }

        egui::Panel::top("top_panel").resizable(false).show_inside(ui, |ui| {
            self.render_top_panel(ui);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if ui.input(|i| i.viewport().close_requested()) {
                if self.allowed_to_close || !self.project.is_dirty {
                } else {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.show_close_dialog = true;
                }
            }

            if self.mode == EditorMode::OldHangulMap && !self.project.old_hangul_enabled {
                self.mode = EditorMode::Drawing;
            }
            match self.mode {
                EditorMode::Drawing => self.ui_drawing_mode(ui),
                EditorMode::Components => self.ui_components_mode(ui),
                EditorMode::Templates => self.ui_templates_mode(ui),
                EditorMode::SelectorRules => self.ui_selector_rules_mode(ui),
                EditorMode::Syllables => self.ui_syllables_mode(ui),
                EditorMode::TypeTest => self.ui_type_test_mode(ui),
                EditorMode::OldHangulMap => self.ui_old_hangul_mode(ui),
            }
        });

        if self.show_close_dialog {
            let s = crate::i18n::t(self.lang);
            egui::Window::new(s.top.unsaved_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(s.top.unsaved_body);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(s.top.save_and_quit).clicked() {
                            self.save_project_now();
                            if !self.project.is_dirty {
                                self.show_close_dialog = false;
                                self.allowed_to_close = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        if ui.button(s.top.discard_and_quit).clicked() {
                            self.show_close_dialog = false;
                            self.allowed_to_close = true;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button(s.top.cancel).clicked() {
                            self.show_close_dialog = false;
                        }
                    });
                });
        }

        self.ui_export_window(&ctx);
        self.ui_discard_dialog(&ctx);
        self.ui_apply_old_hangul_dialog(&ctx);
        self.render_notifications(&ctx);
    }
}

impl FontEditor {
    pub fn compute_layout(&self) -> Option<LayoutResult> {
        self.project
            .engine
            .layout_char(&self.project.store, self.selected_char, decompose_hangul, get_jamo_char)
    }

    pub fn render_context(&self) -> RenderContext<'_> {
        RenderContext {
            store: &self.project.store,
            engine: &self.project.engine,
            canvas_w: self.project.canvas_w,
            canvas_h: self.project.canvas_h,
        }
    }

    pub fn poll_promises(&mut self, _ctx: &egui::Context) {
        if let Some(inner) = self.open_promise.as_ref().and_then(|d| d.take_ready()) {
            self.open_promise = None;
            if let Some((bytes, path)) = inner {
                match parse_project_bytes(&bytes) {
                    Ok(data) => {
                        self.project.current_path = path;
                        self.apply_project(data);
                    }
                    Err(e) => self.push_error(format!("프로젝트 열기 실패: {e}")),
                }
            }
        }

        if let Some(inner) = self.save_as_promise.as_ref().and_then(|d| d.take_ready()) {
            self.save_as_promise = None;
            if let Some(path) = inner {
                self.project.current_path = Some(path);
            }
        }

        if self.export_state.sheet_promise.as_ref().and_then(|d| d.take_ready()).is_some() {
            self.export_state.sheet_promise = None;
        }
        if self.export_state.individual_promise.as_ref().and_then(|d| d.take_ready()).is_some() {
            self.export_state.individual_promise = None;
        }
    }

    pub fn push_error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        self.notifications.push(AppNotification::error(msg));
    }

    pub fn get_glyph_pixels(&self, key: &GlyphKey) -> BTreeSet<(i32, i32)> {
        self.project.store.get(key).map(|g| g.pixels.clone()).unwrap_or_default()
    }

    pub fn copy_pixels_from(&mut self, key: &GlyphKey) {
        self.drawing.pixel_clipboard = Some(self.get_glyph_pixels(key));
    }

    pub fn paste_pixels_to(&mut self, key: &GlyphKey) {
        let Some(clipboard) = self.drawing.pixel_clipboard.clone() else {
            return;
        };
        let current = self.get_glyph_pixels(key);
        if current == clipboard {
            return;
        }
        self.drawing.undo_stack.push((key.clone(), current));
        if self.drawing.undo_stack.len() > 50 {
            self.drawing.undo_stack.remove(0);
        }
        self.drawing.redo_stack.clear();
        match self.project.store.get_mut(key) {
            Some(glyph) => glyph.pixels = clipboard,
            None => {
                self.project.store.glyphs.insert(key.clone(), PixelGlyph { pixels: clipboard });
            }
        }
        self.invalidate_render_caches();
    }

    pub fn undo(&mut self) {
        if let Some((key, pixels)) = self.drawing.undo_stack.pop() {
            self.apply_history_entry(key, pixels, true);
        }
    }

    pub fn redo(&mut self) {
        if let Some((key, pixels)) = self.drawing.redo_stack.pop() {
            self.apply_history_entry(key, pixels, false);
        }
    }

    pub fn invalidate_render_caches(&mut self) {
        self.hangul_browser.pixel_cache.clear();
        self.type_test.pixel_cache.clear();
        self.type_test.triplet_cache.clear();
        self.drawing.preview_dirty = true;
        self.project.is_dirty = true;
        self.components.usage_cache_key = None;
        self.components.group_cache_key = None;
        self.components.archaic_cache_key = None;
        self.components.archaic_group_cache_key = None;
        self.tpl_editor.usage_cache_key = None;
        self.tpl_editor.variant_cache_key = None;
        self.sel_editor.cache_key = None;
    }

    pub fn apply_new_project(&mut self, store: GlyphStore, rules: hangul_syllable::core::rules::RuleSystem) {
        self.project.store = store;
        self.project.engine.rules = rules;
        self.project.current_path = None;
        self.project.is_dirty = false;
        self.allowed_to_close = false;
        self.hangul_browser.reset_on_project_load();
        self.type_test.reset_on_project_load();
        self.drawing.preview_dirty = true;
        self.drawing.undo_stack.clear();
        self.drawing.redo_stack.clear();
        self.tpl_editor.reset_on_project_load();
        self.selected_selector = 0;
        self.sel_editor.reset_on_project_load();
        self.components.reset_on_project_load();
    }

    pub fn apply_project(&mut self, data: ProjectData) {
        self.project.load_data(data);
        self.hangul_browser.reset_on_project_load();
        self.type_test.reset_on_project_load();
        self.drawing.preview_dirty = true;
        self.tpl_editor.reset_on_project_load();
        self.selected_selector = 0;
        self.sel_editor.reset_on_project_load();
        self.components.reset_on_project_load();
        self.allowed_to_close = false;
    }

    fn render_notifications(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        for n in &mut self.notifications {
            if n.expire_at.is_none() {
                n.expire_at = Some(now + 5.0);
            }
        }
        self.notifications.retain(|n| n.expire_at.is_none_or(|t| now < t));

        if self.notifications.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("app_notifications"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                for n in self.notifications.iter().rev().take(5) {
                    let bg = if n.is_error {
                        egui::Color32::from_rgb(180, 40, 40)
                    } else {
                        egui::Color32::from_rgb(160, 110, 0)
                    };
                    egui::Frame::default()
                        .fill(bg)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.colored_label(egui::Color32::WHITE, &n.message);
                        });
                    ui.add_space(4.0);
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    // push_to_redo=true: undo op (save current to redo), false: redo op (save current to undo)
    fn apply_history_entry(&mut self, key: GlyphKey, pixels: BTreeSet<(i32, i32)>, push_to_redo: bool) {
        let current = self.get_glyph_pixels(&key);
        if push_to_redo {
            self.drawing.redo_stack.push((key.clone(), current));
        } else {
            self.drawing.undo_stack.push((key.clone(), current));
        }
        match self.project.store.get_mut(&key) {
            Some(glyph) => glyph.pixels = pixels,
            None => {
                self.project.store.glyphs.insert(key, PixelGlyph { pixels });
            }
        }
        self.invalidate_render_caches();
    }
}

impl FontEditor {
    pub fn ui_discard_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_discard_dialog {
            return;
        }
        let s = crate::i18n::t(self.lang);
        egui::Window::new(s.top.unsaved_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(s.top.discard_body);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(s.top.discard_confirm).clicked() {
                        self.show_discard_dialog = false;
                        if let Some(action) = self.pending_discard.take() {
                            self.execute_discard_action(action, ctx.clone());
                        }
                    }
                    if ui.button(s.top.cancel).clicked() {
                        self.show_discard_dialog = false;
                        self.pending_discard = None;
                    }
                });
            });
    }

    pub fn ui_apply_old_hangul_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_apply_old_hangul_dialog {
            return;
        }
        let s = crate::i18n::t(self.lang);
        egui::Window::new(s.top.apply_old_hangul_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(s.top.apply_old_hangul_body);
                ui.checkbox(&mut self.apply_old_hangul_copy_pixels, s.top.copy_pixels_label);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(s.top.apply_old_hangul_confirm).clicked() {
                        self.show_apply_old_hangul_dialog = false;
                        let map = self.project.archaic_jamo_map.clone();
                        apply_old_hangul_rules(
                            &mut self.project.engine.rules,
                            &mut self.project.store,
                            &map,
                            self.apply_old_hangul_copy_pixels,
                        );
                        self.project.is_dirty = true;
                        self.invalidate_render_caches();
                    }
                    if ui.button(s.top.cancel).clicked() {
                        self.show_apply_old_hangul_dialog = false;
                    }
                });
            });
    }

    pub(super) fn execute_discard_action(&mut self, action: DiscardAction, ctx: egui::Context) {
        use hangul_syllable::core::generator::RuleGenerator;
        match action {
            DiscardAction::NewDefault => {
                let mut store = GlyphStore::default();
                let rules = RuleGenerator::generate_default(&mut store);
                self.apply_new_project(store, rules);
            }
            DiscardAction::NewMinzkn => {
                let mut store = GlyphStore::default();
                let rules = RuleGenerator::generate_10x6x4(&mut store);
                self.apply_new_project(store, rules);
            }
            DiscardAction::NewZik => {
                let mut store = GlyphStore::default();
                let rules = RuleGenerator::generate_zik(&mut store);
                self.apply_new_project(store, rules);
            }
            DiscardAction::NewDkb => {
                let mut store = GlyphStore::default();
                let rules = RuleGenerator::generate_dkb(&mut store);
                self.apply_new_project(store, rules);
            }
            DiscardAction::NewHanterm => {
                let mut store = GlyphStore::default();
                let rules = RuleGenerator::generate_hanterm(&mut store);
                self.apply_new_project(store, rules);
            }
            DiscardAction::OpenFile => {
                self.do_open_project(ctx);
            }
        }
    }

    pub fn open_export_window(&mut self) {
        self.export_state.show_window = true;
    }

    pub fn ui_export_window(&mut self, ctx: &egui::Context) {
        let mut open = self.export_state.show_window;
        let s = crate::i18n::t(self.lang);

        egui::Window::new(s.export.window_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(s.export.target);
                    ui.radio_value(&mut self.export_state.target_type, CharScope::All, s.export.target_all);
                    ui.radio_value(&mut self.export_state.target_type, CharScope::KsX1001, s.export.target_ks);
                    ui.radio_value(&mut self.export_state.target_type, CharScope::AdobeKr9, s.export.target_adobe_kr);
                    ui.radio_value(&mut self.export_state.target_type, CharScope::Custom, s.export.target_custom);
                });

                if self.export_state.target_type == CharScope::Custom {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.export_state.custom_text)
                                .hint_text(s.export.custom_input_hint)
                                .desired_width(300.0),
                        );
                        let count = self
                            .export_state
                            .custom_text
                            .chars()
                            .filter(|&c| (0xAC00..=0xD7A3).contains(&(c as u32)))
                            .collect::<std::collections::HashSet<_>>()
                            .len();
                        ui.label(format!("{}{}", count, s.export.custom_char_count));
                    });
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(s.export.text_color);
                    ui.color_edit_button_rgb(&mut self.export_state.text_color);
                });
                ui.horizontal(|ui| {
                    ui.label(s.export.bg_color);
                    ui.checkbox(&mut self.export_state.bg_transparent, s.export.transparent);
                    if !self.export_state.bg_transparent {
                        ui.color_edit_button_rgb(&mut self.export_state.bg_color);
                    }
                });

                ui.separator();

                ui.columns(2, |cols| {
                    cols[0].group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.strong(s.export.sheet);
                            ui.horizontal(|ui| {
                                ui.label(s.export.sheet_columns);
                                ui.add(egui::DragValue::new(&mut self.export_state.sheet_columns).range(1..=100));
                            });
                            ui.add_space(4.0);
                            let exporting = self.export_state.sheet_promise.is_some();
                            ui.add_enabled_ui(!exporting, |ui| {
                                if ui.button(if exporting { s.export.saving } else { s.export.sheet_save }).clicked() {
                                    self.run_export_sheet();
                                }
                            });
                        });
                    });

                    cols[1].group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.strong(s.export.individual);
                            egui::ComboBox::from_id_salt("export_name_fmt")
                                .selected_text(match self.export_state.name_format {
                                    FileNameFormat::Char => "가.png",
                                    FileNameFormat::Hex => "AC00.png",
                                    FileNameFormat::UHex => "UAC00.png",
                                    FileNameFormat::UPlusHex => "U+AC00.png",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.export_state.name_format, FileNameFormat::Char, "가.png");
                                    ui.selectable_value(&mut self.export_state.name_format, FileNameFormat::Hex, "AC00.png");
                                    ui.selectable_value(&mut self.export_state.name_format, FileNameFormat::UHex, "UAC00.png");
                                    ui.selectable_value(&mut self.export_state.name_format, FileNameFormat::UPlusHex, "U+AC00.png");
                                });
                            ui.add_space(4.0);

                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let exporting = self.export_state.individual_promise.is_some();
                                ui.add_enabled_ui(!exporting, |ui| {
                                    if ui
                                        .button(if exporting { s.export.saving } else { s.export.individual_save })
                                        .clicked()
                                    {
                                        self.run_export_individual();
                                    }
                                });
                            }

                            #[cfg(target_arch = "wasm32")]
                            ui.weak(s.export.web_sheet_only);
                        });
                    });
                });
            });

        self.export_state.show_window = open;
    }

    fn make_export_config(&self) -> ExportConfig {
        let t = self.export_state.text_color;
        let b = self.export_state.bg_color;

        let text_rgba = [(t[0] * 255.0) as u8, (t[1] * 255.0) as u8, (t[2] * 255.0) as u8, 255];
        let bg_rgba = if self.export_state.bg_transparent {
            [0, 0, 0, 0]
        } else {
            [(b[0] * 255.0) as u8, (b[1] * 255.0) as u8, (b[2] * 255.0) as u8, 255]
        };

        ExportConfig {
            canvas_w: self.project.canvas_w as u32,
            canvas_h: self.project.canvas_h as u32,
            color_text: text_rgba,
            color_bg: bg_rgba,
            columns: self.export_state.sheet_columns,
            name_format: self.export_state.name_format,
        }
    }

    fn run_export_sheet(&mut self) {
        let chars = get_char_list(&self.export_state.target_type, &self.export_state.custom_text);
        let cfg = self.make_export_config();
        let image = build_sheet_image(&self.render_context(), &chars, &cfg);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path_opt = rfd::FileDialog::new()
                .set_file_name("font_sheet.png")
                .add_filter("PNG Image", &["png"])
                .save_file();
            if let Some(path) = path_opt {
                self.export_state.sheet_promise = Some(Deferred::spawn_thread(move || {
                    image.save(&path).ok();
                    Some(())
                }));
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            use hangul_syllable::io::export::encode_png;
            crate::platform::trigger_download(&encode_png(&image), "font_sheet.png");
        }
    }

    fn run_export_individual(&mut self) {
        let chars = get_char_list(&self.export_state.target_type, &self.export_state.custom_text);
        let cfg = self.make_export_config();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use hangul_syllable::io::export::render_single_char;
            let dir_opt = rfd::FileDialog::new().pick_folder();
            if let Some(dir) = dir_opt {
                let ctx = self.render_context();
                let images: RgbaImageVec = chars.iter().map(|&ch| (ch, render_single_char(&ctx, ch, &cfg))).collect();
                self.export_state.individual_promise = Some(Deferred::spawn_thread(move || {
                    for (ch, img) in &images {
                        let _ = img.save(dir.join(cfg.name_format.format(*ch)));
                    }
                    Some(())
                }));
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (chars, cfg);
        }
    }
}
