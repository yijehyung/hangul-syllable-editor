use std::collections::BTreeSet;

use crate::core::{glyph::GlyphStore, rules::RuleSystem, types::HangulComponent};

use super::{all_jamo_sets, build_selector, build_template, new_rule_system, no_jong_set};

pub fn generate(store: &mut GlyphStore) -> RuleSystem {
    let mut sys = new_rule_system();

    let v_jung: BTreeSet<char> = "ㅏㅐㅑㅒㅓㅔㅕㅖㅣ".chars().collect();
    let h_jung: BTreeSet<char> = "ㅗㅛㅜㅠㅡ".chars().collect();
    let c_jung: BTreeSet<char> = "ㅘㅙㅚㅝㅞㅟㅢ".chars().collect();

    let (cho_m, _, jong_m) = all_jamo_sets();

    let g_cho_v_no = sys.add_group(store, "받침X_초성_세로", HangulComponent::Cho, cho_m.clone());
    let g_cho_h_no = sys.add_group(store, "받침X_초성_가로", HangulComponent::Cho, cho_m.clone());
    let g_cho_c_no = sys.add_group(store, "받침X_초성_이중", HangulComponent::Cho, cho_m.clone());
    let g_cho_v_with = sys.add_group(store, "받침O_초성_세로", HangulComponent::Cho, cho_m.clone());
    let g_cho_h_with = sys.add_group(store, "받침O_초성_가로", HangulComponent::Cho, cho_m.clone());
    let g_cho_c_with = sys.add_group(store, "받침O_초성_이중", HangulComponent::Cho, cho_m.clone());
    let g_jung_v_no = sys.add_group(store, "받침X_중성_세로", HangulComponent::Jung, v_jung.clone());
    let g_jung_h_no = sys.add_group(store, "받침X_중성_가로", HangulComponent::Jung, h_jung.clone());
    let g_jung_c_no = sys.add_group(store, "받침X_중성_이중", HangulComponent::Jung, c_jung.clone());
    let g_jung_v_with = sys.add_group(store, "받침O_중성_세로", HangulComponent::Jung, v_jung.clone());
    let g_jung_h_with = sys.add_group(store, "받침O_중성_가로", HangulComponent::Jung, h_jung.clone());
    let g_jung_c_with = sys.add_group(store, "받침O_중성_이중", HangulComponent::Jung, c_jung.clone());
    let g_jong_base = sys.add_group(store, "종성_기본", HangulComponent::Jong, jong_m.clone());

    sys.base_cho_group_id = g_cho_v_no.clone();
    sys.base_jung_group_id = g_jung_v_no.clone();
    sys.base_jong_group_id = g_jong_base.clone();

    let no_jong = no_jong_set();

    let tpl_cv_no = sys.new_template_id();
    let tpl_ch_no = sys.new_template_id();
    let tpl_cx_no = sys.new_template_id();
    let tpl_cv_with = sys.new_template_id();
    let tpl_ch_with = sys.new_template_id();
    let tpl_cx_with = sys.new_template_id();

    sys.templates = vec![
        build_template(tpl_cv_no.clone(), "받침X/세로", g_cho_v_no, g_jung_v_no, None, vec![]),
        build_template(tpl_ch_no.clone(), "받침X/가로", g_cho_h_no, g_jung_h_no, None, vec![]),
        build_template(tpl_cx_no.clone(), "받침X/이중", g_cho_c_no, g_jung_c_no, None, vec![]),
        build_template(
            tpl_cv_with.clone(),
            "받침O/세로",
            g_cho_v_with,
            g_jung_v_with,
            Some(g_jong_base.clone()),
            vec![],
        ),
        build_template(
            tpl_ch_with.clone(),
            "받침O/가로",
            g_cho_h_with,
            g_jung_h_with,
            Some(g_jong_base.clone()),
            vec![],
        ),
        build_template(
            tpl_cx_with.clone(),
            "받침O/이중",
            g_cho_c_with,
            g_jung_c_with,
            Some(g_jong_base.clone()),
            vec![],
        ),
    ];

    let s1 = sys.new_selector_id();
    let s2 = sys.new_selector_id();
    let s3 = sys.new_selector_id();
    let s4 = sys.new_selector_id();
    let s5 = sys.new_selector_id();
    let s6 = sys.new_selector_id();

    sys.selectors = vec![
        build_selector(s1, "받침X/세로", v_jung.clone(), false, tpl_cv_no, &jong_m, &no_jong),
        build_selector(s2, "받침X/가로", h_jung.clone(), false, tpl_ch_no, &jong_m, &no_jong),
        build_selector(s3, "받침X/이중", c_jung.clone(), false, tpl_cx_no, &jong_m, &no_jong),
        build_selector(s4, "받침O/세로", v_jung, true, tpl_cv_with, &jong_m, &no_jong),
        build_selector(s5, "받침O/가로", h_jung, true, tpl_ch_with, &jong_m, &no_jong),
        build_selector(s6, "받침O/이중", c_jung, true, tpl_cx_with, &jong_m, &no_jong),
    ];

    sys
}
