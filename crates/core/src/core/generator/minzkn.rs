// UnBitFonts - minzkn님 글꼴 (https://sites.google.com/site/unbitfonts/composite)
use std::collections::BTreeSet;

use crate::core::{
    glyph::GlyphStore,
    rules::{RuleSystem, SelectorRule, Template, VariantRule},
    types::HangulComponent,
};

use super::{add_preset_case, all_jamo_sets, build_variant_rule, new_rule_system, no_jong_set};

pub fn generate(store: &mut GlyphStore) -> RuleSystem {
    let mut sys = new_rule_system();

    let jung_shape1: BTreeSet<char> = "ㅏㅐㅑㅒㅓㅔㅕㅖㅣ".chars().collect();
    let jung_shape2: BTreeSet<char> = "ㅗㅛㅡ".chars().collect();
    let jung_shape3: BTreeSet<char> = "ㅜㅠ".chars().collect();
    let jung_shape4: BTreeSet<char> = "ㅘㅙㅚㅢ".chars().collect();
    let jung_shape5: BTreeSet<char> = "ㅝㅞㅟ".chars().collect();

    let cho_gk: BTreeSet<char> = "ㄱㅋ".chars().collect();
    let cho_h: BTreeSet<char> = "ㅎ".chars().collect();

    let (cho_all, jung_all, jong_all) = all_jamo_sets();

    let gc1 = sys.add_group(store, "받침X_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc2 = sys.add_group(store, "받침X_초성_ㅗㅛㅡ", HangulComponent::Cho, cho_all.clone());
    let gc3 = sys.add_group(store, "받침X_초성_ㅜㅠ", HangulComponent::Cho, cho_all.clone());
    let gc4 = sys.add_group(store, "받침X_초성_ㅘㅙㅚㅢ", HangulComponent::Cho, cho_all.clone());
    let gc5 = sys.add_group(store, "받침X_초성_ㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());
    let gc6 = sys.add_group(store, "받침O_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc7 = sys.add_group(store, "받침O_초성_ㅗㅛㅡ", HangulComponent::Cho, cho_all.clone());
    let gc8 = sys.add_group(store, "받침O_초성_ㅜㅠ", HangulComponent::Cho, cho_all.clone());
    let gc9 = sys.add_group(store, "받침O_초성_ㅘㅙㅚㅢ", HangulComponent::Cho, cho_all.clone());
    let gc10 = sys.add_group(store, "받침O_초성_ㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());

    let gj1 = sys.add_group(store, "받침X_중성_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj2 = sys.add_group(store, "받침X_중성_초성ㅎ", HangulComponent::Jung, jung_all.clone());
    let gj3 = sys.add_group(store, "받침X_중성_초성기타", HangulComponent::Jung, jung_all.clone());
    let gj4 = sys.add_group(store, "받침O_중성_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj5 = sys.add_group(store, "받침O_중성_초성ㅎ", HangulComponent::Jung, jung_all.clone());
    let gj6 = sys.add_group(store, "받침O_중성_초성기타", HangulComponent::Jung, jung_all.clone());

    let gjong1 = sys.add_group(store, "종성_ㅏㅑㅘㅣ", HangulComponent::Jong, jong_all.clone());
    let gjong2 = sys.add_group(store, "종성_ㅓㅕㅚㅝㅟㅢ", HangulComponent::Jong, jong_all.clone());
    let gjong3 = sys.add_group(store, "종성_ㅐㅒㅔㅖㅙㅞ", HangulComponent::Jong, jong_all.clone());
    let gjong4 = sys.add_group(store, "종성_ㅗㅛㅜㅠㅡ", HangulComponent::Jong, jong_all.clone());

    sys.base_cho_group_id = gc1.clone();
    sys.base_jung_group_id = gj3.clone();
    sys.base_jong_group_id = gjong1.clone();

    let all_group_ids: Vec<String> = sys.groups.iter().map(|g| g.id.clone()).collect();
    for gid in all_group_ids {
        sys.ensure_group_glyphs(store, &gid);
    }

    let no_jong = no_jong_set();

    let make_jung_variants = |has_jong: bool| -> Vec<VariantRule> {
        if !has_jong {
            vec![
                build_variant_rule(
                    "초성[ㄱㅋ] → 중성그룹1",
                    Some(&cho_gk),
                    None,
                    None,
                    100,
                    None,
                    Some(gj1.as_str()),
                    None,
                ),
                build_variant_rule("초성[ㅎ] → 중성그룹2", Some(&cho_h), None, None, 99, None, Some(gj2.as_str()), None),
            ]
        } else {
            vec![
                build_variant_rule(
                    "초성[ㄱㅋ] → 중성그룹4",
                    Some(&cho_gk),
                    None,
                    None,
                    100,
                    None,
                    Some(gj4.as_str()),
                    None,
                ),
                build_variant_rule("초성[ㅎ] → 중성그룹5", Some(&cho_h), None, None, 99, None, Some(gj5.as_str()), None),
            ]
        }
    };

    let mut templates: Vec<Template> = Vec::new();
    let mut selectors: Vec<SelectorRule> = Vec::new();

    for (name, jung_set, cho_group, jong_group) in [
        ("받침X/ㅏ계열", jung_shape1.clone(), &gc1, None),
        ("받침X/ㅗㅛㅡ", jung_shape2.clone(), &gc2, None),
        ("받침X/ㅜㅠ", jung_shape3.clone(), &gc3, None),
        ("받침X/ㅘㅙㅚㅢ", jung_shape4.clone(), &gc4, None),
        ("받침X/ㅝㅞㅟ", jung_shape5.clone(), &gc5, None),
    ] {
        add_preset_case(
            &mut sys,
            &mut templates,
            &mut selectors,
            name,
            jung_set,
            false,
            cho_group,
            &gj3,
            jong_group,
            make_jung_variants(false),
            &jong_all,
            &no_jong,
        );
    }

    for (name, jung_set, cho_group, jong_group) in [
        ("받침O/ㅏㅑㅣ", "ㅏㅑㅣ".chars().collect::<BTreeSet<_>>(), &gc6, &gjong1),
        ("받침O/ㅓㅕ", "ㅓㅕ".chars().collect(), &gc6, &gjong2),
        ("받침O/ㅐㅒㅔㅖ", "ㅐㅒㅔㅖ".chars().collect(), &gc6, &gjong3),
        ("받침O/ㅗㅛㅡ", "ㅗㅛㅡ".chars().collect(), &gc7, &gjong4),
        ("받침O/ㅜㅠ", "ㅜㅠ".chars().collect(), &gc8, &gjong4),
        ("받침O/ㅘ", "ㅘ".chars().collect(), &gc9, &gjong1),
        ("받침O/ㅚㅢ", "ㅚㅢ".chars().collect(), &gc9, &gjong2),
        ("받침O/ㅙ", "ㅙ".chars().collect(), &gc9, &gjong3),
        ("받침O/ㅝㅟ", "ㅝㅟ".chars().collect(), &gc10, &gjong2),
        ("받침O/ㅞ", "ㅞ".chars().collect(), &gc10, &gjong3),
    ] {
        add_preset_case(
            &mut sys,
            &mut templates,
            &mut selectors,
            name,
            jung_set,
            true,
            cho_group,
            &gj6,
            Some(jong_group.as_str()),
            make_jung_variants(true),
            &jong_all,
            &no_jong,
        );
    }

    sys.templates = templates;
    sys.selectors = selectors;
    sys
}
