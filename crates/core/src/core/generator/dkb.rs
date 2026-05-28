// ZIK님 GMS 한글 조합 렌더링 (https://github.com/TandyRum1024/hangul-johab-render-gms)
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

    let s1: BTreeSet<char> = "ㅏㅐㅑㅒㅓㅔㅕㅖㅣ".chars().collect();
    let s2: BTreeSet<char> = "ㅗㅛㅡ".chars().collect();
    let s3: BTreeSet<char> = "ㅜㅠ".chars().collect();
    let s4: BTreeSet<char> = "ㅘㅙㅚㅢ".chars().collect();
    let s5: BTreeSet<char> = "ㅝㅞㅟ".chars().collect();

    let gc0 = sys.add_group(store, "받침X_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc1 = sys.add_group(store, "받침X_초성_ㅗㅛㅡ", HangulComponent::Cho, cho_all.clone());
    let gc2 = sys.add_group(store, "받침X_초성_ㅜㅠ", HangulComponent::Cho, cho_all.clone());
    let gc3 = sys.add_group(store, "받침X_초성_ㅘㅙㅚㅢ", HangulComponent::Cho, cho_all.clone());
    let gc4 = sys.add_group(store, "받침X_초성_ㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());
    let gc5 = sys.add_group(store, "받침O_초성_ㅏ계열", HangulComponent::Cho, cho_all.clone());
    let gc6 = sys.add_group(store, "받침O_초성_ㅗㅛㅜㅠㅡ", HangulComponent::Cho, cho_all.clone());
    let gc7 = sys.add_group(store, "받침O_초성_ㅘㅙㅚㅢㅝㅞㅟ", HangulComponent::Cho, cho_all.clone());

    let gj0 = sys.add_group(store, "받침X_중성_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj1 = sys.add_group(store, "받침X_중성_초성기타", HangulComponent::Jung, jung_all.clone());
    let gj2 = sys.add_group(store, "받침O_중성_초성ㄱㅋ", HangulComponent::Jung, jung_all.clone());
    let gj3 = sys.add_group(store, "받침O_중성_초성기타", HangulComponent::Jung, jung_all.clone());

    let gjong0 = sys.add_group(store, "종성_ㅏㅑㅘ", HangulComponent::Jong, jong_all.clone());
    let gjong1 = sys.add_group(store, "종성_ㅓㅕㅚㅝㅟㅢㅣ", HangulComponent::Jong, jong_all.clone());
    let gjong2 = sys.add_group(store, "종성_ㅐㅒㅔㅖㅙㅞ", HangulComponent::Jong, jong_all.clone());
    let gjong3 = sys.add_group(store, "종성_ㅗㅛㅜㅠㅡ", HangulComponent::Jong, jong_all.clone());

    sys.base_cho_group_id = gc0.clone();
    sys.base_jung_group_id = gj1.clone();
    sys.base_jong_group_id = gjong0.clone();

    let no_jong = no_jong_set();

    let jung_vr_no = || {
        vec![build_variant_rule(
            "초성[ㄱㅋ] → 중성그룹1",
            Some(&cho_gk),
            None,
            None,
            100,
            None,
            Some(gj0.as_str()),
            None,
        )]
    };
    let jung_vr_with = || {
        vec![build_variant_rule(
            "초성[ㄱㅋ] → 중성그룹3",
            Some(&cho_gk),
            None,
            None,
            100,
            None,
            Some(gj2.as_str()),
            None,
        )]
    };

    let mut templates: Vec<Template> = Vec::new();
    let mut selectors: Vec<SelectorRule> = Vec::new();

    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/ㅏ계열",
        s1.clone(),
        false,
        &gc0,
        &gj1,
        None,
        jung_vr_no(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/ㅗㅛㅡ",
        s2.clone(),
        false,
        &gc1,
        &gj1,
        None,
        jung_vr_no(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/ㅜㅠ",
        s3.clone(),
        false,
        &gc2,
        &gj1,
        None,
        jung_vr_no(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/ㅘㅙㅚㅢ",
        s4.clone(),
        false,
        &gc3,
        &gj1,
        None,
        jung_vr_no(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침X/ㅝㅞㅟ",
        s5.clone(),
        false,
        &gc4,
        &gj1,
        None,
        jung_vr_no(),
        &jong_all,
        &no_jong,
    );

    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅏㅑ",
        "ㅏㅑ".chars().collect(),
        true,
        &gc5,
        &gj3,
        Some(gjong0.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅓㅕㅣ",
        "ㅓㅕㅣ".chars().collect(),
        true,
        &gc5,
        &gj3,
        Some(gjong1.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅐㅒㅔㅖ",
        "ㅐㅒㅔㅖ".chars().collect(),
        true,
        &gc5,
        &gj3,
        Some(gjong2.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅗㅛㅡ",
        "ㅗㅛㅡ".chars().collect(),
        true,
        &gc6,
        &gj3,
        Some(gjong3.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅜㅠ",
        "ㅜㅠ".chars().collect(),
        true,
        &gc6,
        &gj3,
        Some(gjong3.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅘ",
        "ㅘ".chars().collect(),
        true,
        &gc7,
        &gj3,
        Some(gjong0.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅚㅢ",
        "ㅚㅢ".chars().collect(),
        true,
        &gc7,
        &gj3,
        Some(gjong1.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅙ",
        "ㅙ".chars().collect(),
        true,
        &gc7,
        &gj3,
        Some(gjong2.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅝㅟ",
        "ㅝㅟ".chars().collect(),
        true,
        &gc7,
        &gj3,
        Some(gjong1.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );
    add_preset_case(
        &mut sys,
        &mut templates,
        &mut selectors,
        "받침O/ㅞ",
        "ㅞ".chars().collect(),
        true,
        &gc7,
        &gj3,
        Some(gjong2.as_str()),
        jung_vr_with(),
        &jong_all,
        &no_jong,
    );

    sys.templates = templates;
    sys.selectors = selectors;
    sys
}
