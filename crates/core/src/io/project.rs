use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::{
    engine::LayoutEngine,
    glyph::{GlyphKey, GlyphStore, PixelGlyph},
    hangul::{all_hangul_syllables, allowed_chars_extended, allowed_chars_for_target, decompose_hangul, get_jamo_char},
    rules::RuleSystem,
    types::HangulComponent,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectData {
    pub version: String,
    pub canvas_w: i32,
    pub canvas_h: i32,
    pub store: GlyphStore,
    pub rules: RuleSystem,
    #[serde(default)]
    pub old_hangul_enabled: bool,
    #[serde(
        default = "crate::core::old_hangul::default_archaic_map",
        skip_serializing_if = "is_default_archaic_map"
    )]
    pub archaic_jamo_map: Vec<(char, char)>,
}

impl ProjectData {
    pub fn from_editor(
        canvas_w: i32,
        canvas_h: i32,
        store: &GlyphStore,
        rules: &RuleSystem,
        old_hangul_enabled: bool,
        archaic_jamo_map: Vec<(char, char)>,
    ) -> Self {
        Self {
            version: "1.0.0".to_string(),
            canvas_w,
            canvas_h,
            store: store.clone(),
            rules: rules.clone(),
            old_hangul_enabled,
            archaic_jamo_map,
        }
    }
}

fn is_default_archaic_map(map: &[(char, char)]) -> bool {
    map == crate::core::old_hangul::ARCHAIC_JAMO_MAP
}

#[derive(Serialize, Deserialize)]
struct ProjectDataFile {
    version: String,
    canvas_w: i32,
    canvas_h: i32,
    store: GlyphStoreFile,
    rules: RuleSystem,
    #[serde(default)]
    old_hangul_enabled: bool,
    #[serde(
        default = "crate::core::old_hangul::default_archaic_map",
        skip_serializing_if = "is_default_archaic_map"
    )]
    archaic_jamo_map: Vec<(char, char)>,
}

#[derive(Serialize, Deserialize)]
struct GlyphStoreFile {
    glyphs: BTreeMap<String, PixelRowsFile>,
}

#[derive(Serialize, Deserialize)]
struct PixelRowsFile {
    pixels: String,
}

fn key_to_str(kind: HangulComponent, jamo: char, gid: &str) -> String {
    format!("{}/{}/{}", kind.kind(), jamo, gid)
}

fn key_from_str(s: &str) -> Option<(HangulComponent, char, String)> {
    let mut parts = s.splitn(3, '/');
    let kind_u8: u8 = parts.next()?.parse().ok()?;
    let kind = HangulComponent::from_kind(kind_u8)?;
    let jamo_str = parts.next()?;
    let gid = parts.next()?.to_string();
    let jamo = jamo_str.chars().next()?;
    Some((kind, jamo, gid))
}

fn pixels_to_str(pixels: &BTreeSet<(i32, i32)>, canvas_w: i32, canvas_h: i32) -> String {
    let hex_digits = ((canvas_w + 3) / 4) as usize;
    let bit_count = (hex_digits * 4) as i32;
    (0..canvas_h)
        .map(|y| {
            let mask: u64 = (0..canvas_w)
                .filter(|&x| pixels.contains(&(x, y)))
                .fold(0u64, |acc, x| acc | (1u64 << (bit_count - 1 - x)));
            format!("{:0>width$x}", mask, width = hex_digits)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn rows_to_pixels(s: &str) -> BTreeSet<(i32, i32)> {
    let mut set = BTreeSet::new();
    for (y, token) in s.split_whitespace().enumerate() {
        if let Ok(mask) = u64::from_str_radix(token, 16) {
            let bit_count = token.len() * 4;
            for bit in 0..bit_count {
                if mask & (1u64 << (bit_count - 1 - bit)) != 0 {
                    set.insert((bit as i32, y as i32));
                }
            }
        }
    }
    set
}

fn to_file_format(data: &ProjectData) -> ProjectDataFile {
    let glyphs = data
        .store
        .glyphs
        .iter()
        .map(|(key, glyph)| {
            let k = key_to_str(key.kind, key.jamo, &key.group_id);
            let pixels = pixels_to_str(&glyph.pixels, data.canvas_w, data.canvas_h);
            (k, PixelRowsFile { pixels })
        })
        .collect();

    ProjectDataFile {
        version: "1.0.0".to_string(),
        canvas_w: data.canvas_w,
        canvas_h: data.canvas_h,
        store: GlyphStoreFile { glyphs },
        rules: data.rules.clone(),
        old_hangul_enabled: data.old_hangul_enabled,
        archaic_jamo_map: data.archaic_jamo_map.clone(),
    }
}

fn from_file_format(file: ProjectDataFile) -> ProjectData {
    let glyphs: BTreeMap<GlyphKey, PixelGlyph> = file
        .store
        .glyphs
        .into_iter()
        .filter_map(|(key_str, rows_file)| {
            let (kind, jamo, gid) = key_from_str(&key_str)?;
            let pixels = rows_to_pixels(&rows_file.pixels);
            Some((GlyphKey::new(kind, jamo, gid), PixelGlyph { pixels }))
        })
        .collect();

    ProjectData {
        version: file.version,
        canvas_w: file.canvas_w,
        canvas_h: file.canvas_h,
        store: GlyphStore { glyphs },
        rules: file.rules,
        old_hangul_enabled: file.old_hangul_enabled,
        archaic_jamo_map: file.archaic_jamo_map,
    }
}

fn scan_used_glyphs_by_layout(rules: &mut RuleSystem, store: &GlyphStore) -> HashSet<GlyphKey> {
    let taken = std::mem::take(rules);
    let engine = LayoutEngine { rules: taken };
    let mut used: HashSet<GlyphKey> = HashSet::new();

    for ch in all_hangul_syllables() {
        let Some(res) = engine.layout_char(store, ch, decompose_hangul, get_jamo_char) else {
            continue;
        };

        used.insert(GlyphKey::new(
            res.cho.placement.kind,
            res.cho.placement.jamo,
            res.cho.group_id.clone(),
        ));
        used.insert(GlyphKey::new(
            res.jung.placement.kind,
            res.jung.placement.jamo,
            res.jung.group_id.clone(),
        ));
        if let Some(jong) = res.jong.as_ref() {
            used.insert(GlyphKey::new(jong.placement.kind, jong.placement.jamo, jong.group_id.clone()));
        }
    }

    *rules = engine.rules;
    used
}

fn build_allowed_set(kind: HangulComponent, old_hangul_enabled: bool) -> HashSet<char> {
    if old_hangul_enabled {
        allowed_chars_extended(kind).into_iter().collect()
    } else {
        allowed_chars_for_target(kind).iter().copied().collect()
    }
}

fn sanitize_group_members(rules: &mut RuleSystem, old_hangul_enabled: bool) {
    let mut removed_total: usize = 0;
    for g in &mut rules.groups {
        let before = g.members.len();
        let allowed = build_allowed_set(g.target, old_hangul_enabled);
        g.members.retain(|ch| allowed.contains(ch));
        removed_total += before.saturating_sub(g.members.len());
    }
    if removed_total > 0 {
        log::warn!("[sanitize] ⚠ groups.members에서 유효하지 않은 자모 {}개 제거됨", removed_total);
    }
}

fn sanitize_base_groups(rules: &mut RuleSystem, group_id_set: &HashSet<String>) {
    let fallback_group =
        |target: HangulComponent| -> Option<String> { rules.groups.iter().find(|g| g.target == target).map(|g| g.id.clone()) };
    if !group_id_set.contains(&rules.base_cho_group_id)
        && let Some(id) = fallback_group(HangulComponent::Cho)
    {
        log::warn!(
            "[sanitize] ⚠ base_cho_group_id '{}' 없음 → '{}' 로 교체",
            rules.base_cho_group_id,
            id
        );
        rules.base_cho_group_id = id;
    }
    if !group_id_set.contains(&rules.base_jung_group_id)
        && let Some(id) = fallback_group(HangulComponent::Jung)
    {
        log::warn!(
            "[sanitize] ⚠ base_jung_group_id '{}' 없음 → '{}' 로 교체",
            rules.base_jung_group_id,
            id
        );
        rules.base_jung_group_id = id;
    }
    if !group_id_set.contains(&rules.base_jong_group_id)
        && let Some(id) = fallback_group(HangulComponent::Jong)
    {
        log::warn!(
            "[sanitize] ⚠ base_jong_group_id '{}' 없음 → '{}' 로 교체",
            rules.base_jong_group_id,
            id
        );
        rules.base_jong_group_id = id;
    }
}

fn sanitize_template_references(rules: &mut RuleSystem, group_id_set: &HashSet<String>) {
    let base_cho = rules.base_cho_group_id.clone();
    let base_jung = rules.base_jung_group_id.clone();
    let base_jong = rules.base_jong_group_id.clone();

    let mut fixed_tpl = 0usize;
    let mut fixed_vr = 0usize;
    for t in &mut rules.templates {
        if !group_id_set.contains(&t.default_cho_group_id) {
            t.default_cho_group_id = base_cho.clone();
            fixed_tpl += 1;
        }
        if !group_id_set.contains(&t.default_jung_group_id) {
            t.default_jung_group_id = base_jung.clone();
            fixed_tpl += 1;
        }
        if let Some(gid) = &t.default_jong_group_id
            && !group_id_set.contains(gid)
        {
            t.default_jong_group_id = Some(base_jong.clone());
            fixed_tpl += 1;
        }
        for vr in &mut t.variant_rules {
            if let Some(gid) = vr.set_cho_group_id.as_ref()
                && !group_id_set.contains(gid)
            {
                vr.set_cho_group_id = Some(base_cho.clone());
                fixed_vr += 1;
            }
            if let Some(gid) = vr.set_jung_group_id.as_ref()
                && !group_id_set.contains(gid)
            {
                vr.set_jung_group_id = Some(base_jung.clone());
                fixed_vr += 1;
            }
            if let Some(gid) = vr.set_jong_group_id.as_ref()
                && !group_id_set.contains(gid)
            {
                vr.set_jong_group_id = Some(base_jong.clone());
                fixed_vr += 1;
            }
        }
    }
    if fixed_tpl > 0 {
        log::warn!("[sanitize] ⚠ template default group 참조 {}건을 Base로 보정", fixed_tpl);
    }
    if fixed_vr > 0 {
        log::warn!("[sanitize] ⚠ template variant_rules group 참조 {}건을 Base로 보정", fixed_vr);
    }
}

fn sanitize_selector_references(rules: &mut RuleSystem) {
    let mut fixed_sel = 0usize;
    if let Some(first_tpl) = rules.templates.first().map(|t| t.id.clone()) {
        let tpl_id_set: HashSet<String> = rules.templates.iter().map(|t| t.id.clone()).collect();
        for s in &mut rules.selectors {
            if !tpl_id_set.contains(&s.template_id) {
                s.template_id = first_tpl.clone();
                fixed_sel += 1;
            }
        }
    }
    if fixed_sel > 0 {
        log::warn!("[sanitize] ⚠ selectors.template_id {}건을 첫 템플릿으로 보정", fixed_sel);
    }
}

fn sanitize_glyphs(store: &mut GlyphStore, rules: &mut RuleSystem, old_hangul_enabled: bool) {
    let used = scan_used_glyphs_by_layout(rules, store);

    let allowed_cho = build_allowed_set(HangulComponent::Cho, old_hangul_enabled);
    let allowed_jung = build_allowed_set(HangulComponent::Jung, old_hangul_enabled);
    let allowed_jong = build_allowed_set(HangulComponent::Jong, old_hangul_enabled);

    let mut group_map: HashMap<String, (HangulComponent, HashSet<char>, HashSet<char>)> = HashMap::new();
    for g in &rules.groups {
        let allowed_set = match g.target {
            HangulComponent::Cho => allowed_cho.clone(),
            HangulComponent::Jung => allowed_jung.clone(),
            HangulComponent::Jong => allowed_jong.clone(),
        };
        group_map.insert(g.id.clone(), (g.target, allowed_set, g.members.iter().copied().collect()));
    }

    let before = store.glyphs.len();
    let mut removed_no_group = 0usize;
    let mut removed_kind_mismatch = 0usize;
    let mut removed_jamo_invalid = 0usize;
    let mut removed_unused = 0usize;
    let mut fixed_missing_member: Vec<GlyphKey> = Vec::new();

    store.glyphs.retain(|key, _| {
        let Some((expected_kind, allowed_set, members_set)) = group_map.get(&key.group_id) else {
            removed_no_group += 1;
            return false;
        };
        if key.kind != *expected_kind {
            removed_kind_mismatch += 1;
            return false;
        }
        if !allowed_set.contains(&key.jamo) {
            removed_jamo_invalid += 1;
            return false;
        }

        if members_set.contains(&key.jamo) {
            return true;
        }

        if used.contains(key) {
            fixed_missing_member.push(key.clone());
            return true;
        }

        removed_unused += 1;
        false
    });

    for key in &fixed_missing_member {
        if let Some(g) = rules.groups.iter_mut().find(|g| g.id == key.group_id) {
            g.members.insert(key.jamo);
        }
    }

    let removed = before.saturating_sub(store.glyphs.len());
    if removed > 0 {
        log::warn!(
            "[sanitize] ⚠ store.glyphs 정리: {}개 제거됨 ({} -> {})",
            removed,
            before,
            store.glyphs.len()
        );
        if removed_no_group > 0 {
            log::warn!("[sanitize]   - group 없음: {}개", removed_no_group);
        }
        if removed_kind_mismatch > 0 {
            log::warn!("[sanitize]   - kind mismatch: {}개", removed_kind_mismatch);
        }
        if removed_jamo_invalid > 0 {
            log::warn!("[sanitize]   - 허용 자모 외: {}개", removed_jamo_invalid);
        }
        if removed_unused > 0 {
            log::warn!("[sanitize]   - 미사용: {}개", removed_unused);
        }
    }
    if !fixed_missing_member.is_empty() {
        log::warn!("[sanitize] group.members 누락 자모 {}개 자동 추가됨", fixed_missing_member.len());
        for key in fixed_missing_member.iter().take(12) {
            log::warn!("[sanitize]   (kind={:?}, jamo='{}', gid={})", key.kind, key.jamo, key.group_id);
        }
    }
}

pub(crate) fn sanitize_rules_and_store(rules: &mut RuleSystem, store: &mut GlyphStore, old_hangul_enabled: bool) {
    sanitize_group_members(rules, old_hangul_enabled);

    let group_id_set: HashSet<String> = rules.groups.iter().map(|g| g.id.clone()).collect();

    sanitize_base_groups(rules, &group_id_set);
    sanitize_template_references(rules, &group_id_set);
    sanitize_selector_references(rules);
    sanitize_glyphs(store, rules, old_hangul_enabled);
}

pub fn compact_ids(data: &mut ProjectData) {
    let remap = |map: &HashMap<String, String>, id: &str| -> String { map.get(id).cloned().unwrap_or_else(|| id.to_string()) };

    let group_map: HashMap<String, String> = data
        .rules
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.id.clone(), format!("grp_{:06}", i)))
        .collect();

    let tpl_map: HashMap<String, String> = data
        .rules
        .templates
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.clone(), format!("tpl_{:06}", i)))
        .collect();

    let sel_map: HashMap<String, String> = data
        .rules
        .selectors
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), format!("sel_{:06}", i)))
        .collect();

    let mut vr_seq = 0u64;
    let mut vr_map: HashMap<String, String> = HashMap::new();
    for t in &data.rules.templates {
        for vr in &t.variant_rules {
            vr_map.insert(vr.id.clone(), format!("vr_{:06}", vr_seq));
            vr_seq += 1;
        }
    }

    for g in &mut data.rules.groups {
        g.id = remap(&group_map, &g.id);
    }

    data.rules.base_cho_group_id = remap(&group_map, &data.rules.base_cho_group_id);
    data.rules.base_jung_group_id = remap(&group_map, &data.rules.base_jung_group_id);
    data.rules.base_jong_group_id = remap(&group_map, &data.rules.base_jong_group_id);

    for t in &mut data.rules.templates {
        t.id = remap(&tpl_map, &t.id);
        t.default_cho_group_id = remap(&group_map, &t.default_cho_group_id);
        t.default_jung_group_id = remap(&group_map, &t.default_jung_group_id);
        t.default_jong_group_id = t.default_jong_group_id.as_deref().map(|id| remap(&group_map, id));
        for vr in &mut t.variant_rules {
            vr.id = remap(&vr_map, &vr.id);
            vr.set_cho_group_id = vr.set_cho_group_id.as_deref().map(|id| remap(&group_map, id));
            vr.set_jung_group_id = vr.set_jung_group_id.as_deref().map(|id| remap(&group_map, id));
            vr.set_jong_group_id = vr.set_jong_group_id.as_deref().map(|id| remap(&group_map, id));
        }
    }

    for s in &mut data.rules.selectors {
        s.id = remap(&sel_map, &s.id);
        s.template_id = remap(&tpl_map, &s.template_id);
    }

    let old_glyphs = std::mem::take(&mut data.store.glyphs);
    data.store.glyphs = old_glyphs
        .into_iter()
        .map(|(key, v)| (GlyphKey::new(key.kind, key.jamo, remap(&group_map, &key.group_id)), v))
        .collect();

    let (n_grp, n_tpl, n_sel) = (
        data.rules.groups.len() as u64,
        data.rules.templates.len() as u64,
        data.rules.selectors.len() as u64,
    );
    data.rules.reset_seqs(n_grp, n_tpl, n_sel, vr_seq);
}

pub fn parse_project_bytes(bytes: &[u8]) -> Result<ProjectData, String> {
    let file = serde_yaml::from_slice::<ProjectDataFile>(bytes).map_err(|e| format!("YAML 파싱 실패: {e}"))?;
    let mut data = from_file_format(file);
    sanitize_rules_and_store(&mut data.rules, &mut data.store, data.old_hangul_enabled);
    Ok(data)
}

pub fn serialize_project_to_yaml(data: &mut ProjectData) -> Result<String, String> {
    sanitize_rules_and_store(&mut data.rules, &mut data.store, data.old_hangul_enabled);
    compact_ids(data);
    let file = to_file_format(data);
    serde_yaml::to_string(&file).map_err(|e| format!("YAML 직렬화 실패: {e}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_project_to_path(path: &Path, data: &mut ProjectData) -> Result<(), String> {
    sanitize_rules_and_store(&mut data.rules, &mut data.store, data.old_hangul_enabled);
    compact_ids(data);
    let file = to_file_format(data);
    let yaml_str = serde_yaml::to_string(&file).map_err(|e| format!("YAML 직렬화 실패: {e}"))?;
    std::fs::write(path, yaml_str).map_err(|e| format!("파일 쓰기 실패: {e}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_project_from_path(path: &Path) -> Result<ProjectData, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("파일 읽기 실패: {e}"))?;
    parse_project_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        generator::RuleGenerator,
        glyph::{GlyphKey, GlyphStore},
    };

    fn roundtrip(mut data: ProjectData) -> ProjectData {
        let yaml = serialize_project_to_yaml(&mut data).expect("직렬화 실패");
        parse_project_bytes(yaml.as_bytes()).expect("역직렬화 실패")
    }

    fn empty_project() -> ProjectData {
        ProjectData {
            version: "1.0.0".to_string(),
            canvas_w: 16,
            canvas_h: 16,
            store: GlyphStore::default(),
            rules: RuleSystem::default(),
            old_hangul_enabled: false,
            archaic_jamo_map: crate::core::old_hangul::default_archaic_map(),
        }
    }

    fn default_project() -> ProjectData {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_default(&mut store);
        ProjectData {
            version: "1.0.0".to_string(),
            canvas_w: 16,
            canvas_h: 16,
            store,
            rules,
            old_hangul_enabled: false,
            archaic_jamo_map: crate::core::old_hangul::default_archaic_map(),
        }
    }

    #[test]
    fn roundtrip_empty_project_ok() {
        roundtrip(empty_project());
    }

    #[test]
    fn roundtrip_preserves_canvas_size() {
        let mut data = empty_project();
        data.canvas_w = 24;
        data.canvas_h = 32;
        let back = roundtrip(data);
        assert_eq!(back.canvas_w, 24);
        assert_eq!(back.canvas_h, 32);
    }

    #[test]
    fn roundtrip_default_rules_preserves_group_count() {
        let data = default_project();
        let group_count = data.rules.groups.len();
        let back = roundtrip(data);
        assert_eq!(back.rules.groups.len(), group_count);
    }

    #[test]
    fn roundtrip_default_rules_preserves_template_count() {
        let data = default_project();
        let tpl_count = data.rules.templates.len();
        let back = roundtrip(data);
        assert_eq!(back.rules.templates.len(), tpl_count);
    }

    #[test]
    fn roundtrip_preserves_pixels() {
        use crate::core::types::HangulComponent;
        let mut data = default_project();
        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &data.rules.groups[0].id);
        data.store.ensure_glyph(key.clone());
        data.store.get_mut(&key).unwrap().set(3, 5);
        let back = roundtrip(data);
        let compact_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &back.rules.groups[0].id);
        assert!(back.store.get(&compact_key).unwrap().pixels.contains(&(3, 5)));
    }

    #[test]
    fn hex_roundtrip_non_multiple_of_4_canvas_w() {
        // canvas_w=13 (4의 배수 아님): 이전 코드에서 x좌표가 3칸 shift되는 버그
        use crate::core::types::HangulComponent;
        let mut data = default_project();
        data.canvas_w = 13;
        data.canvas_h = 16;
        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &data.rules.groups[0].id);
        data.store.ensure_glyph(key.clone());
        data.store.get_mut(&key).unwrap().set(0, 0);
        data.store.get_mut(&key).unwrap().set(12, 3);
        let back = roundtrip(data);
        let compact_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &back.rules.groups[0].id);
        let pixels = &back.store.get(&compact_key).unwrap().pixels;
        assert!(pixels.contains(&(0, 0)), "x=0 픽셀이 복원되어야 함, 실제: {pixels:?}");
        assert!(pixels.contains(&(12, 3)), "x=12 픽셀이 복원되어야 함, 실제: {pixels:?}");
        assert!(!pixels.contains(&(3, 0)), "shift된 좌표 x=3이 없어야 함");
    }

    #[test]
    fn hex_roundtrip_canvas_w_10() {
        // canvas_w=10 (shift=2)
        use crate::core::types::HangulComponent;
        let mut data = default_project();
        data.canvas_w = 10;
        data.canvas_h = 10;
        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &data.rules.groups[0].id);
        data.store.ensure_glyph(key.clone());
        data.store.get_mut(&key).unwrap().set(0, 0);
        data.store.get_mut(&key).unwrap().set(9, 0);
        let back = roundtrip(data);
        let compact_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', &back.rules.groups[0].id);
        let pixels = &back.store.get(&compact_key).unwrap().pixels;
        assert!(pixels.contains(&(0, 0)), "x=0 픽셀이 복원되어야 함");
        assert!(pixels.contains(&(9, 0)), "x=9 픽셀이 복원되어야 함");
    }

    #[test]
    fn roundtrip_sanitize_removes_orphan_glyphs() {
        use crate::core::types::HangulComponent;
        let mut data = empty_project();
        let orphan_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "nonexistent_group");
        data.store.ensure_glyph(orphan_key.clone());
        data.store.get_mut(&orphan_key).unwrap().set(1, 1);
        let back = roundtrip(data);
        assert!(!back.store.has(&orphan_key));
    }

    #[test]
    fn serialize_produces_yaml_string() {
        let mut data = empty_project();
        let yaml = serialize_project_to_yaml(&mut data).unwrap();
        assert!(yaml.contains("canvas_w"));
        assert!(yaml.contains("version"));
    }

    use crate::core::{
        groups::ComponentGroup,
        rules::{SelectorRule, Template, VariantRule},
        types::HangulComponent,
    };

    fn make_group(id: &str, target: HangulComponent) -> ComponentGroup {
        ComponentGroup {
            id: id.into(),
            name: id.into(),
            target,
            members: Default::default(),
        }
    }

    fn make_template(id: &str, cho: &str, jung: &str, jong: Option<&str>) -> Template {
        Template {
            id: id.into(),
            name: id.into(),
            default_cho_group_id: cho.into(),
            default_jung_group_id: jung.into(),
            default_jong_group_id: jong.map(Into::into),
            variant_rules: vec![],
        }
    }

    fn make_selector(id: &str, tpl: &str) -> SelectorRule {
        use crate::core::rules::CharSetCond;
        SelectorRule {
            id: id.into(),
            name: id.into(),
            priority: 0,
            cho_set: CharSetCond::Any,
            jung_set: CharSetCond::Any,
            jong_set: CharSetCond::Any,
            template_id: tpl.into(),
        }
    }

    fn make_variant(id: &str, cho: Option<&str>, jung: Option<&str>, jong: Option<&str>) -> VariantRule {
        use crate::core::rules::CharSetCond;
        VariantRule {
            id: id.into(),
            name: id.into(),
            priority: 0,
            cho_set: CharSetCond::Any,
            jung_set: CharSetCond::Any,
            jong_set: CharSetCond::Any,
            set_cho_group_id: cho.map(Into::into),
            set_jung_group_id: jung.map(Into::into),
            set_jong_group_id: jong.map(Into::into),
        }
    }

    fn project_with_gaps() -> ProjectData {
        let cho = make_group("grp_000005", HangulComponent::Cho);
        let jung = make_group("grp_000010", HangulComponent::Jung);
        let jong = make_group("grp_000099", HangulComponent::Jong);

        let mut tpl = make_template("tpl_000003", "grp_000005", "grp_000010", Some("grp_000099"));
        tpl.variant_rules
            .push(make_variant("vr_000007", Some("grp_000005"), Some("grp_000010"), None));

        let sel = make_selector("sel_000020", "tpl_000003");

        let mut rules = RuleSystem::default();
        rules.groups = vec![cho, jung, jong];
        rules.templates = vec![tpl];
        rules.selectors = vec![sel];
        rules.base_cho_group_id = "grp_000005".into();
        rules.base_jung_group_id = "grp_000010".into();
        rules.base_jong_group_id = "grp_000099".into();
        rules.reset_seqs(100, 50, 30, 20);
        let mut data = ProjectData {
            version: "1.0.0".into(),
            canvas_w: 12,
            canvas_h: 12,
            store: GlyphStore::default(),
            rules,
            old_hangul_enabled: false,
            archaic_jamo_map: crate::core::old_hangul::default_archaic_map(),
        };

        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "grp_000005");
        data.store.ensure_glyph(key.clone());
        data.store.get_mut(&key).unwrap().set(3, 7);

        data
    }

    #[test]
    fn compact_groups_renamed_sequentially() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let ids: Vec<&str> = data.rules.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["grp_000000", "grp_000001", "grp_000002"]);
    }

    #[test]
    fn compact_templates_renamed_sequentially() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.templates[0].id, "tpl_000000");
    }

    #[test]
    fn compact_selectors_renamed_sequentially() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.selectors[0].id, "sel_000000");
    }

    #[test]
    fn compact_variant_rules_renamed_sequentially() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.templates[0].variant_rules[0].id, "vr_000000");
    }

    #[test]
    fn compact_base_group_ids_updated() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.base_cho_group_id, "grp_000000");
        assert_eq!(data.rules.base_jung_group_id, "grp_000001");
        assert_eq!(data.rules.base_jong_group_id, "grp_000002");
    }

    #[test]
    fn compact_template_default_group_ids_updated() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let t = &data.rules.templates[0];
        assert_eq!(t.default_cho_group_id, "grp_000000");
        assert_eq!(t.default_jung_group_id, "grp_000001");
        assert_eq!(t.default_jong_group_id.as_deref(), Some("grp_000002"));
    }

    #[test]
    fn compact_variant_rule_group_ids_updated() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let vr = &data.rules.templates[0].variant_rules[0];
        assert_eq!(vr.set_cho_group_id.as_deref(), Some("grp_000000"));
        assert_eq!(vr.set_jung_group_id.as_deref(), Some("grp_000001"));
        assert_eq!(vr.set_jong_group_id, None);
    }

    #[test]
    fn compact_selector_template_id_updated() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.selectors[0].template_id, "tpl_000000");
    }

    #[test]
    fn compact_glyph_store_keys_remapped() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let new_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "grp_000000");
        let old_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "grp_000005");
        assert!(data.store.has(&new_key), "새 키로 접근 가능해야 함");
        assert!(!data.store.has(&old_key), "옛 키는 사라져야 함");
    }

    #[test]
    fn compact_pixel_data_preserved() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let new_key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "grp_000000");
        assert!(data.store.get(&new_key).unwrap().pixels.contains(&(3, 7)));
    }

    #[test]
    fn compact_seq_counters_reset_to_entity_count() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        assert_eq!(data.rules.new_group_id(), "grp_000003");
        assert_eq!(data.rules.new_template_id(), "tpl_000001");
        assert_eq!(data.rules.new_selector_id(), "sel_000001");
        assert_eq!(data.rules.new_variant_id(), "vr_000001");
    }

    #[test]
    fn compact_new_ids_after_load_dont_collide() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let next_grp = data.rules.new_group_id();
        let all_ids: Vec<&str> = data.rules.groups.iter().map(|g| g.id.as_str()).collect();
        assert!(!all_ids.contains(&next_grp.as_str()));
    }

    #[test]
    fn compact_idempotent() {
        let mut data = project_with_gaps();
        compact_ids(&mut data);
        let ids_after_first: Vec<String> = data.rules.groups.iter().map(|g| g.id.clone()).collect();
        compact_ids(&mut data);
        let ids_after_second: Vec<String> = data.rules.groups.iter().map(|g| g.id.clone()).collect();
        assert_eq!(ids_after_first, ids_after_second);
    }

    #[test]
    fn compact_empty_project_does_not_panic() {
        let mut data = empty_project();
        compact_ids(&mut data);
        assert!(data.rules.groups.is_empty());
        assert_eq!(data.rules.new_group_id(), "grp_000000");
    }

    #[test]
    fn compact_multiple_templates_variant_seq_global() {
        let cho = make_group("grp_0", HangulComponent::Cho);
        let jung = make_group("grp_1", HangulComponent::Jung);
        let jong = make_group("grp_2", HangulComponent::Jong);
        let mut tpl_a = make_template("tpl_A", "grp_0", "grp_1", None);
        tpl_a.variant_rules.push(make_variant("vr_X", None, None, None));
        tpl_a.variant_rules.push(make_variant("vr_Y", None, None, None));
        let mut tpl_b = make_template("tpl_B", "grp_0", "grp_1", None);
        tpl_b.variant_rules.push(make_variant("vr_Z", None, None, None));
        tpl_b.variant_rules.push(make_variant("vr_W", None, None, None));
        let mut rules = RuleSystem::default();
        rules.groups = vec![cho, jung, jong];
        rules.templates = vec![tpl_a, tpl_b];
        rules.base_cho_group_id = "grp_0".into();
        rules.base_jung_group_id = "grp_1".into();
        rules.base_jong_group_id = "grp_2".into();
        let mut data = ProjectData {
            version: "1.0.0".into(),
            canvas_w: 12,
            canvas_h: 12,
            store: GlyphStore::default(),
            rules,
            old_hangul_enabled: false,
            archaic_jamo_map: crate::core::old_hangul::default_archaic_map(),
        };
        compact_ids(&mut data);
        assert_eq!(data.rules.templates[0].variant_rules[0].id, "vr_000000");
        assert_eq!(data.rules.templates[0].variant_rules[1].id, "vr_000001");
        assert_eq!(data.rules.templates[1].variant_rules[0].id, "vr_000002");
        assert_eq!(data.rules.templates[1].variant_rules[1].id, "vr_000003");
        assert_eq!(data.rules.new_variant_id(), "vr_000004");
    }

    #[test]
    fn compact_roundtrip_ids_remain_sequential() {
        let mut data = project_with_gaps();
        let yaml = serialize_project_to_yaml(&mut data).unwrap();
        let loaded = parse_project_bytes(yaml.as_bytes()).unwrap();
        assert_eq!(loaded.rules.groups[0].id, "grp_000000");
        assert_eq!(loaded.rules.groups[1].id, "grp_000001");
        assert_eq!(loaded.rules.templates[0].id, "tpl_000000");
        assert_eq!(loaded.rules.selectors[0].id, "sel_000000");
    }

    #[test]
    fn compact_roundtrip_pixel_data_intact() {
        let mut data = project_with_gaps();
        let yaml = serialize_project_to_yaml(&mut data).unwrap();
        let loaded = parse_project_bytes(yaml.as_bytes()).unwrap();
        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "grp_000000");
        assert!(loaded.store.get(&key).unwrap().pixels.contains(&(3, 7)));
    }
}
