use crate::core::{
    glyph::{GlyphKey, GlyphStore},
    hangul::NO_JONG,
    rules::{CharSetCond, RuleSystem, SelectorRule, Template, VariantRule},
    types::{HangulComponent, kind_to_name},
};

const MSG_DEFAULT_TEMPLATE: &str = "(기본 템플릿)";
const MSG_JONG_SLOT_MISSING: &str = "문자에 종성이 있는데, 선택 템플릿에 종성 슬롯이 없음";

#[derive(Clone, Debug)]
pub struct FallbackEntry {
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub struct PartPlacement {
    pub kind: HangulComponent,
    pub jamo: char,
}

#[derive(Clone, Debug)]
pub struct PlacedPart {
    pub placement: PartPlacement,
    pub group_id: String,
}

#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub template_id: String,
    pub template_name: String,
    pub selector_name: String,
    pub matched_variants: Vec<String>,
    pub cho: PlacedPart,
    pub jung: PlacedPart,
    pub jong: Option<PlacedPart>,
    pub fallbacks: Vec<FallbackEntry>,
}

#[derive(Default)]
pub struct LayoutEngine {
    pub rules: RuleSystem,
}

impl LayoutEngine {
    fn selector_matches(s: &SelectorRule, cho_ch: char, jung_ch: char, jong_ch: Option<char>) -> bool {
        jamo_sets_match(&s.cho_set, &s.jung_set, &s.jong_set, cho_ch, jung_ch, jong_ch)
    }

    fn pick_template(&self, cho_ch: char, jung_ch: char, jong_ch: Option<char>) -> Option<(&Template, String)> {
        let mut sel_refs: Vec<&SelectorRule> = self.rules.selectors.iter().collect();
        sel_refs.sort_unstable_by_key(|s| std::cmp::Reverse(s.priority));

        for s in sel_refs {
            if Self::selector_matches(s, cho_ch, jung_ch, jong_ch)
                && let Some(tpl) = self.rules.find_template(&s.template_id)
            {
                let name = if s.name.is_empty() { s.id.clone() } else { s.name.clone() };
                return Some((tpl, name));
            }
        }

        self.rules.templates.first().map(|t| (t, MSG_DEFAULT_TEMPLATE.to_string()))
    }

    fn apply_variant_rules_group(
        tpl: &Template,
        cho_ch: char,
        jung_ch: char,
        jong_ch: Option<char>,
        mut gids: GroupIds,
    ) -> (GroupIds, Vec<String>) {
        let mut rule_refs: Vec<&VariantRule> = tpl.variant_rules.iter().collect();
        rule_refs.sort_unstable_by_key(|r| r.priority);

        let mut matched: Vec<String> = Vec::new();

        for r in rule_refs {
            if !jamo_sets_match(&r.cho_set, &r.jung_set, &r.jong_set, cho_ch, jung_ch, jong_ch) {
                continue;
            }
            matched.push(if r.name.is_empty() { r.id.clone() } else { r.name.clone() });
            if let Some(gid) = &r.set_cho_group_id {
                gids.cho = gid.clone();
            }
            if let Some(gid) = &r.set_jung_group_id {
                gids.jung = gid.clone();
            }
            if let Some(gid) = &r.set_jong_group_id {
                gids.jong = gid.clone();
            }
        }

        (gids, matched)
    }

    fn resolve_group_id_for_part(
        rules: &RuleSystem,
        store: &GlyphStore,
        kind: HangulComponent,
        jamo: char,
        desired_gid: &str,
    ) -> (String, Vec<FallbackEntry>) {
        let group_exists = rules.find_group(desired_gid).is_some();
        let glyph_exists = store.has(&GlyphKey::new(kind, jamo, desired_gid));

        if group_exists && glyph_exists {
            return (desired_gid.to_string(), vec![]);
        }

        let part_name = kind_to_name(kind);
        let message = if group_exists {
            format!("{part_name} '{jamo}' 에서 group '{desired_gid}' 글리프 없음 -> Base로 fallback")
        } else {
            format!("{part_name} '{jamo}' 에서 group '{desired_gid}' 존재하지 않음 -> Base로 fallback")
        };

        let mut fallbacks = vec![FallbackEntry { message }];

        let base = rules.base_group_id_for_kind(kind).to_string();
        if store.has(&GlyphKey::new(kind, jamo, &base)) {
            return (base, fallbacks);
        }

        fallbacks.push(FallbackEntry {
            message: format!("{part_name} '{jamo}' 에서 Base group '{base}' 글리프도 없음"),
        });
        (desired_gid.to_string(), fallbacks)
    }

    pub fn layout_jamo(&self, store: &GlyphStore, cho_ch: char, jung_ch: char, jong_ch: Option<char>) -> Option<LayoutResult> {
        let (tpl, sel_dbg) = self.pick_template(cho_ch, jung_ch, jong_ch)?;

        let gids = GroupIds {
            cho: tpl.default_cho_group_id.clone(),
            jung: tpl.default_jung_group_id.clone(),
            jong: tpl.default_jong_group_id.clone().unwrap_or_default(),
        };

        let (gids, matched_variants) = Self::apply_variant_rules_group(tpl, cho_ch, jung_ch, jong_ch, gids);

        let mut fallbacks = vec![];

        let (resolved_cho_gid, cho_f) = Self::resolve_group_id_for_part(&self.rules, store, HangulComponent::Cho, cho_ch, &gids.cho);
        fallbacks.extend(cho_f);
        let (resolved_jung_gid, jung_f) = Self::resolve_group_id_for_part(&self.rules, store, HangulComponent::Jung, jung_ch, &gids.jung);
        fallbacks.extend(jung_f);

        let cho = PlacedPart {
            placement: PartPlacement {
                kind: HangulComponent::Cho,
                jamo: cho_ch,
            },
            group_id: resolved_cho_gid,
        };
        let jung = PlacedPart {
            placement: PartPlacement {
                kind: HangulComponent::Jung,
                jamo: jung_ch,
            },
            group_id: resolved_jung_gid,
        };

        let jong = if let Some(jong_ch) = jong_ch {
            if tpl.default_jong_group_id.is_some() {
                let (resolved_jong_gid, jong_f) =
                    Self::resolve_group_id_for_part(&self.rules, store, HangulComponent::Jong, jong_ch, &gids.jong);
                fallbacks.extend(jong_f);
                Some(PlacedPart {
                    placement: PartPlacement {
                        kind: HangulComponent::Jong,
                        jamo: jong_ch,
                    },
                    group_id: resolved_jong_gid,
                })
            } else {
                fallbacks.push(FallbackEntry {
                    message: MSG_JONG_SLOT_MISSING.to_string(),
                });
                None
            }
        } else {
            None
        };

        Some(LayoutResult {
            template_id: tpl.id.clone(),
            template_name: if tpl.name.is_empty() { tpl.id.clone() } else { tpl.name.clone() },
            selector_name: sel_dbg,
            matched_variants,
            cho,
            jung,
            jong,
            fallbacks,
        })
    }

    pub fn layout_char(
        &self,
        store: &GlyphStore,
        c: char,
        decompose: impl Fn(char) -> Option<(usize, usize, usize)>,
        get_jamo_char: impl Fn(HangulComponent, usize) -> char,
    ) -> Option<LayoutResult> {
        let (cho_idx, jung_idx, jong_idx) = decompose(c)?;
        let cho_ch = get_jamo_char(HangulComponent::Cho, cho_idx);
        let jung_ch = get_jamo_char(HangulComponent::Jung, jung_idx);
        let jong_ch = (jong_idx != 0).then(|| get_jamo_char(HangulComponent::Jong, jong_idx));
        self.layout_jamo(store, cho_ch, jung_ch, jong_ch)
    }
}

struct GroupIds {
    cho: String,
    jung: String,
    jong: String,
}

fn jamo_sets_match(
    cho_set: &CharSetCond,
    jung_set: &CharSetCond,
    jong_set: &CharSetCond,
    cho_ch: char,
    jung_ch: char,
    jong_ch: Option<char>,
) -> bool {
    cho_set.matches(cho_ch) && jung_set.matches(jung_ch) && jong_set.matches(jong_ch.unwrap_or(NO_JONG))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        generator::RuleGenerator,
        glyph::GlyphStore,
        hangul::{decompose_hangul, get_jamo_char},
        rules::{CharSetCond, SelectorRule, Template},
    };

    fn default_engine() -> (LayoutEngine, GlyphStore) {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_default(&mut store);
        (LayoutEngine { rules }, store)
    }

    fn layout(engine: &LayoutEngine, store: &GlyphStore, ch: char) -> Option<LayoutResult> {
        engine.layout_char(store, ch, decompose_hangul, get_jamo_char)
    }

    #[test]
    fn layout_ascii_returns_none() {
        let (eng, store) = default_engine();
        assert!(layout(&eng, &store, 'A').is_none());
    }

    #[test]
    fn layout_no_templates_returns_none() {
        let eng = LayoutEngine::default();
        let store = GlyphStore::default();
        assert!(layout(&eng, &store, '가').is_none());
    }

    #[test]
    fn layout_ga_has_no_jong() {
        let (eng, store) = default_engine();
        let res = layout(&eng, &store, '가').unwrap();
        assert!(res.jong.is_none());
    }

    #[test]
    fn layout_gal_has_jong() {
        let (eng, store) = default_engine();
        let res = layout(&eng, &store, '갈').unwrap();
        let jong = res.jong.unwrap();
        assert_eq!(jong.placement.jamo, 'ㄹ');
    }

    #[test]
    fn layout_ga_cho_is_giyeok() {
        let (eng, store) = default_engine();
        let res = layout(&eng, &store, '가').unwrap();
        assert_eq!(res.cho.placement.jamo, 'ㄱ');
    }

    #[test]
    fn layout_ga_jung_is_a() {
        let (eng, store) = default_engine();
        let res = layout(&eng, &store, '가').unwrap();
        assert_eq!(res.jung.placement.jamo, 'ㅏ');
    }

    #[test]
    fn layout_ga_vs_gal_different_templates() {
        let (eng, store) = default_engine();
        let no_jong = layout(&eng, &store, '가').unwrap();
        let with_jong = layout(&eng, &store, '갈').unwrap();
        assert_ne!(no_jong.template_id, with_jong.template_id);
    }

    #[test]
    fn layout_empty_store_produces_fallbacks() {
        let mut store = GlyphStore::default();
        let rules = RuleGenerator::generate_default(&mut store);
        let eng = LayoutEngine { rules };
        let empty_store = GlyphStore::default();
        let res = layout(&eng, &empty_store, '가').unwrap();
        assert!(!res.fallbacks.is_empty());
    }

    #[test]
    fn layout_no_fallbacks_when_glyphs_present() {
        let (eng, store) = default_engine();
        let res = layout(&eng, &store, '가').unwrap();
        assert!(res.fallbacks.is_empty());
    }

    #[test]
    fn layout_variant_applied_changes_group_id() {
        let mut store = GlyphStore::default();
        let mut rules = RuleGenerator::generate_default(&mut store);

        let variant_gid = "variant_cho".to_string();
        let variant_id = rules.new_variant_id();
        let giyeok_set: std::collections::BTreeSet<char> = ['ㄱ'].into_iter().collect();
        let vr = crate::core::rules::VariantRule {
            id: variant_id,
            name: "giyeok_variant".to_string(),
            priority: 1,
            cho_set: CharSetCond::Include(giyeok_set),
            jung_set: CharSetCond::Any,
            jong_set: CharSetCond::Any,
            set_cho_group_id: Some(variant_gid.clone()),
            set_jung_group_id: None,
            set_jong_group_id: None,
        };
        if let Some(tpl) = rules.templates.first_mut() {
            tpl.variant_rules.push(vr);
        }

        let eng = LayoutEngine { rules };
        let res = layout(&eng, &store, '가').unwrap();
        assert!(res.matched_variants.contains(&"giyeok_variant".to_string()));
    }

    #[test]
    fn layout_single_selector_engine() {
        let mut rules = RuleSystem::default();
        let tpl_id = rules.new_template_id();
        let sel_id = rules.new_selector_id();

        let cho_members: std::collections::BTreeSet<char> = ['ㄱ'].into_iter().collect();
        let jung_members: std::collections::BTreeSet<char> = ['ㅏ'].into_iter().collect();
        let mut store = GlyphStore::default();
        let cho_gid = rules.add_group(&mut store, "cho", crate::core::types::HangulComponent::Cho, cho_members);
        let jung_gid = rules.add_group(&mut store, "jung", crate::core::types::HangulComponent::Jung, jung_members);
        rules.base_cho_group_id = cho_gid.clone();
        rules.base_jung_group_id = jung_gid.clone();

        rules.add_template(Template {
            id: tpl_id.clone(),
            name: "test".to_string(),
            default_cho_group_id: cho_gid,
            default_jung_group_id: jung_gid,
            default_jong_group_id: None,
            variant_rules: vec![],
        });
        rules.add_selector(SelectorRule {
            id: sel_id,
            name: "any".to_string(),
            priority: 0,
            cho_set: CharSetCond::Any,
            jung_set: CharSetCond::Any,
            jong_set: CharSetCond::Any,
            template_id: tpl_id,
        });

        let eng = LayoutEngine { rules };
        assert!(layout(&eng, &store, '가').is_some());
    }
}
