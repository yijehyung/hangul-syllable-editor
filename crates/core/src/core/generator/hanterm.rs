// UnBitFonts - hanterm 글꼴 (https://sites.google.com/site/unbitfonts/composite)
use std::collections::BTreeSet;

use crate::core::{
    glyph::GlyphStore,
    rules::{RuleSystem, SelectorRule, Template},
    types::HangulComponent,
};

use super::{add_preset_case, all_jamo_sets, build_variant_rule, new_rule_system, no_jong_set};

pub fn generate(store: &mut GlyphStore) -> RuleSystem {
    let mut sys = new_rule_system();

    let (cho_all, jung_all, jong_all) = all_jamo_sets();
    let cho_gk: BTreeSet<char> = "ㄱㅋ".chars().collect();

    let gc0 = sys.add_group(store, "받침X_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc1 = sys.add_group(store, "받침X_초성_ㅗㅛㅡ", HangulComponent::Cho, cho_all.clone());
    let gc2 = sys.add_group(store, "받침X_초성_ㅜㅠ", HangulComponent::Cho, cho_all.clone());
    let gc3 = sys.add_group(store, "받침X_초성_ㅘㅙㅚㅢ", HangulComponent::Cho, cho_all.clone());
    let gc4 = sys.add_group(store, "받침X_초성_ㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());
    let gc5 = sys.add_group(store, "받침O_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc6 = sys.add_group(store, "받침O_초성_ㅗㅛㅡ", HangulComponent::Cho, cho_all.clone());
    let gc7 = sys.add_group(store, "받침O_초성_ㅜㅠ", HangulComponent::Cho, cho_all.clone());
    let gc8 = sys.add_group(store, "받침O_초성_ㅘㅙㅚㅢ", HangulComponent::Cho, cho_all.clone());
    let gc9 = sys.add_group(store, "받침O_초성_ㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());

    let gj0 = sys.add_group(store, "받침X_중성_세로", HangulComponent::Jung, jung_all.clone());
    let gj1 = sys.add_group(store, "받침ㄴ_중성_세로", HangulComponent::Jung, jung_all.clone());
    let gj2 = sys.add_group(store, "받침기타_중성_세로", HangulComponent::Jung, jung_all.clone());
    let gj3 = sys.add_group(store, "받침X_중성_가로_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj4 = sys.add_group(store, "받침X_중성_가로_초성기타", HangulComponent::Jung, jung_all.clone());
    let gj5 = sys.add_group(store, "받침O_중성_가로_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj6 = sys.add_group(store, "받침O_중성_가로_초성기타", HangulComponent::Jung, jung_all.clone());

    let gjong0 = sys.add_group(store, "종성_ㅏㅑㅘㅣ", HangulComponent::Jong, jong_all.clone());
    let gjong1 = sys.add_group(store, "종성_ㅓㅕㅚㅝㅟㅢ", HangulComponent::Jong, jong_all.clone());
    let gjong2 = sys.add_group(store, "종성_ㅐㅒㅔㅖㅙㅞ", HangulComponent::Jong, jong_all.clone());
    let gjong3 = sys.add_group(store, "종성_ㅗㅛㅜㅠㅡ", HangulComponent::Jong, jong_all.clone());

    sys.base_cho_group_id = gc0.clone();
    sys.base_jung_group_id = gj0.clone();
    sys.base_jong_group_id = gjong0.clone();

    let no_jong = no_jong_set();
    let jong_n_set: BTreeSet<char> = ['ㄴ'].into_iter().collect();

    let variant_jong_n = || {
        vec![build_variant_rule(
            "종성[ㄴ] → 중성그룹2",
            None,
            None,
            Some(&jong_n_set),
            100,
            None,
            Some(gj1.as_str()),
            None,
        )]
    };
    let variant_h_gk_no = || {
        vec![build_variant_rule(
            "초성[ㄱㅋ] → 중성그룹4",
            Some(&cho_gk),
            None,
            None,
            100,
            None,
            Some(gj3.as_str()),
            None,
        )]
    };
    let variant_h_gk_with = || {
        vec![build_variant_rule(
            "초성[ㄱㅋ] → 중성그룹6",
            Some(&cho_gk),
            None,
            None,
            100,
            None,
            Some(gj5.as_str()),
            None,
        )]
    };

    let mut templates: Vec<Template> = Vec::new();
    let mut selectors: Vec<SelectorRule> = Vec::new();

    // 받침X 세로
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/세로/ㅏ계열",
        "ㅏㅐㅑㅒㅓㅔㅕㅖㅣ".chars().collect(),
        false,
        &gc0,
        &gj0,
        None,
        vec![],
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/세로/ㅜㅠ",
        "ㅜㅠ".chars().collect(),
        false,
        &gc2,
        &gj0,
        None,
        vec![],
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/세로/ㅝㅞㅟ",
        "ㅝㅞㅟ".chars().collect(),
        false,
        &gc4,
        &gj0,
        None,
        vec![],
        &jong_all,
        &no_jong,
    );

    // 받침O 세로
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅏㅑㅣ",
        "ㅏㅑㅣ".chars().collect(),
        true,
        &gc5,
        &gj2,
        Some(gjong0.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅓㅕ",
        "ㅓㅕ".chars().collect(),
        true,
        &gc5,
        &gj2,
        Some(gjong1.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅐㅒㅔㅖ",
        "ㅐㅒㅔㅖ".chars().collect(),
        true,
        &gc5,
        &gj2,
        Some(gjong2.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅜㅠ",
        "ㅜㅠ".chars().collect(),
        true,
        &gc7,
        &gj2,
        Some(gjong3.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅝㅟ",
        "ㅝㅟ".chars().collect(),
        true,
        &gc9,
        &gj2,
        Some(gjong1.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/세로/ㅞ",
        "ㅞ".chars().collect(),
        true,
        &gc9,
        &gj2,
        Some(gjong2.as_str()),
        variant_jong_n(),
        &jong_all,
        &no_jong,
    );

    // 받침X 가로
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/가로/ㅗㅛㅡ",
        "ㅗㅛㅡ".chars().collect(),
        false,
        &gc1,
        &gj4,
        None,
        variant_h_gk_no(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/가로/ㅘㅙㅚㅢ",
        "ㅘㅙㅚㅢ".chars().collect(),
        false,
        &gc3,
        &gj4,
        None,
        variant_h_gk_no(),
        &jong_all,
        &no_jong,
    );

    // 받침O 가로
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/가로/ㅗㅛㅡ",
        "ㅗㅛㅡ".chars().collect(),
        true,
        &gc6,
        &gj6,
        Some(gjong3.as_str()),
        variant_h_gk_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/가로/ㅘ",
        "ㅘ".chars().collect(),
        true,
        &gc8,
        &gj6,
        Some(gjong0.as_str()),
        variant_h_gk_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/가로/ㅚㅢ",
        "ㅚㅢ".chars().collect(),
        true,
        &gc8,
        &gj6,
        Some(gjong1.as_str()),
        variant_h_gk_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/가로/ㅙ",
        "ㅙ".chars().collect(),
        true,
        &gc8,
        &gj6,
        Some(gjong2.as_str()),
        variant_h_gk_with(),
        &jong_all,
        &no_jong,
    );

    sys.templates = templates;
    sys.selectors = selectors;
    sys
}
