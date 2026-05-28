mod default;
mod dkb;
mod hanterm;
mod minzkn;
mod zik;

use std::collections::BTreeSet;

use crate::core::{
    glyph::GlyphStore,
    hangul::{NO_JONG, cho_allowed, jong_allowed, jung_allowed},
    rules::{CharSetCond, RuleSystem, SelectorRule, Template, VariantRule},
};

pub struct RuleGenerator;

impl RuleGenerator {
    pub fn generate_default(store: &mut GlyphStore) -> RuleSystem {
        default::generate(store)
    }

    pub fn generate_10x6x4(store: &mut GlyphStore) -> RuleSystem {
        minzkn::generate(store)
    }

    pub fn generate_zik(store: &mut GlyphStore) -> RuleSystem {
        zik::generate(store)
    }

    pub fn generate_dkb(store: &mut GlyphStore) -> RuleSystem {
        dkb::generate(store)
    }

    pub fn generate_hanterm(store: &mut GlyphStore) -> RuleSystem {
        hanterm::generate(store)
    }
}

pub(super) fn new_rule_system() -> RuleSystem {
    let mut rs = RuleSystem::default();
    rs.reset_seqs(1, 1, 1, 1);
    rs
}

pub(super) fn all_jamo_sets() -> (BTreeSet<char>, BTreeSet<char>, BTreeSet<char>) {
    (
        cho_allowed().iter().copied().collect(),
        jung_allowed().iter().copied().collect(),
        jong_allowed().iter().copied().collect(),
    )
}

pub(super) fn no_jong_set() -> BTreeSet<char> {
    [NO_JONG].into_iter().collect()
}

pub(super) fn build_template(
    id: String,
    name: &str,
    cho: String,
    jung: String,
    jong: Option<String>,
    variant_rules: Vec<VariantRule>,
) -> Template {
    Template {
        id,
        name: name.into(),
        default_cho_group_id: cho,
        default_jung_group_id: jung,
        default_jong_group_id: jong,
        variant_rules,
    }
}

pub(super) fn build_selector(
    id: String,
    name: &str,
    jung_set: BTreeSet<char>,
    has_jong: bool,
    tpl_id: String,
    jong_all: &BTreeSet<char>,
    no_jong: &BTreeSet<char>,
) -> SelectorRule {
    SelectorRule {
        id,
        name: name.into(),
        priority: 100,
        cho_set: CharSetCond::Any,
        jung_set: CharSetCond::Include(jung_set),
        jong_set: if has_jong {
            CharSetCond::Include(jong_all.clone())
        } else {
            CharSetCond::Include(no_jong.clone())
        },
        template_id: tpl_id,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_variant_rule(
    name: &str,
    cho_s: Option<&BTreeSet<char>>,
    jung_s: Option<&BTreeSet<char>>,
    jong_s: Option<&BTreeSet<char>>,
    prio: i32,
    set_cho: Option<&str>,
    set_jung: Option<&str>,
    set_jong: Option<&str>,
) -> VariantRule {
    VariantRule {
        id: name.to_string(),
        name: name.to_string(),
        priority: prio,
        cho_set: cho_s.map(|s| CharSetCond::Include(s.clone())).unwrap_or(CharSetCond::Any),
        jung_set: jung_s.map(|s| CharSetCond::Include(s.clone())).unwrap_or(CharSetCond::Any),
        jong_set: jong_s.map(|s| CharSetCond::Include(s.clone())).unwrap_or(CharSetCond::Any),
        set_cho_group_id: set_cho.map(str::to_string),
        set_jung_group_id: set_jung.map(str::to_string),
        set_jong_group_id: set_jong.map(str::to_string),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn add_preset_case(
    sys: &mut RuleSystem,
    templates: &mut Vec<Template>,
    selectors: &mut Vec<SelectorRule>,
    name: &str,
    jung_set: BTreeSet<char>,
    has_jong: bool,
    cho_group: &str,
    jung_group: &str,
    jong_group: Option<&str>,
    variant_rules: Vec<VariantRule>,
    jong_all: &BTreeSet<char>,
    no_jong: &BTreeSet<char>,
) {
    let tpl_id = sys.new_template_id();
    let sel_id = sys.new_selector_id();
    templates.push(build_template(
        tpl_id.clone(),
        name,
        cho_group.to_string(),
        jung_group.to_string(),
        jong_group.map(str::to_string),
        variant_rules,
    ));
    selectors.push(build_selector(sel_id, name, jung_set, has_jong, tpl_id, jong_all, no_jong));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        engine::LayoutEngine,
        hangul::{decompose_hangul, get_jamo_char},
    };

    fn can_layout(rules: RuleSystem, store: &GlyphStore, ch: char) -> bool {
        let eng = LayoutEngine { rules };
        eng.layout_char(store, ch, decompose_hangul, get_jamo_char).is_some()
    }

    #[test]
    fn generate_default_layouts_all_syllables() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_default(&mut store);
        let eng = LayoutEngine { rules };
        let failed: Vec<char> = crate::core::hangul::all_hangul_syllables()
            .filter(|&ch| eng.layout_char(&store, ch, decompose_hangul, get_jamo_char).is_none())
            .collect();
        assert!(failed.is_empty(), "레이아웃 실패: {:?}", &failed[..failed.len().min(10)]);
    }

    #[test]
    fn generate_10x6x4_layouts_ga() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_10x6x4(&mut store);
        assert!(can_layout(rules, &store, '가'));
    }

    #[test]
    fn generate_zik_layouts_ga() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_zik(&mut store);
        assert!(can_layout(rules, &store, '가'));
    }

    #[test]
    fn generate_dkb_layouts_ga() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_dkb(&mut store);
        assert!(can_layout(rules, &store, '가'));
    }

    #[test]
    fn generate_hanterm_layouts_ga() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_hanterm(&mut store);
        assert!(can_layout(rules, &store, '가'));
    }
}
