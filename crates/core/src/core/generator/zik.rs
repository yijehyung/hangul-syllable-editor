// ZIK님 GMS 한글 조합 렌더링 (https://github.com/TandyRum1024/hangul-johab-render-gms)
use std::collections::BTreeSet;

use crate::core::{glyph::GlyphStore, rules::RuleSystem, types::HangulComponent};

use super::{all_jamo_sets, build_selector, build_template, new_rule_system, no_jong_set};

pub fn generate(store: &mut GlyphStore) -> RuleSystem {
    let mut sys = new_rule_system();

    let (cho_all, jung_all, jong_all) = all_jamo_sets();

    let v_jung: BTreeSet<char> = "ㅏㅐㅑㅒㅓㅔㅕㅖㅣ".chars().collect();
    let h_jung: BTreeSet<char> = "ㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢ".chars().collect();
    let no_jong = no_jong_set();

    let gc_v_no = sys.add_group(store, "받침X_초성_세로", HangulComponent::Cho, cho_all.clone());
    let gc_h_no = sys.add_group(store, "받침X_초성_가로", HangulComponent::Cho, cho_all.clone());
    let gc_v_with = sys.add_group(store, "받침O_초성_세로", HangulComponent::Cho, cho_all.clone());
    let gc_h_with = sys.add_group(store, "받침O_초성_가로", HangulComponent::Cho, cho_all.clone());
    let gj_no = sys.add_group(store, "받침X_중성", HangulComponent::Jung, jung_all.clone());
    let gj_with = sys.add_group(store, "받침O_중성", HangulComponent::Jung, jung_all.clone());
    let gjong_v = sys.add_group(store, "종성_세로", HangulComponent::Jong, jong_all.clone());
    let gjong_h = sys.add_group(store, "종성_가로", HangulComponent::Jong, jong_all.clone());

    sys.base_cho_group_id = gc_v_no.clone();
    sys.base_jung_group_id = gj_no.clone();
    sys.base_jong_group_id = gjong_v.clone();

    let tpl: Vec<String> = (0..4).map(|_| sys.new_template_id()).collect();
    let sel: Vec<String> = (0..4).map(|_| sys.new_selector_id()).collect();

    sys.templates = vec![
        build_template(tpl[0].clone(), "받침X/세로", gc_v_no, gj_no.clone(), None, vec![]),
        build_template(tpl[1].clone(), "받침X/가로", gc_h_no, gj_no, None, vec![]),
        build_template(tpl[2].clone(), "받침O/세로", gc_v_with, gj_with.clone(), Some(gjong_v), vec![]),
        build_template(tpl[3].clone(), "받침O/가로", gc_h_with, gj_with, Some(gjong_h), vec![]),
    ];
    sys.selectors = vec![
        build_selector(
            sel[0].clone(),
            "받침X/세로",
            v_jung.clone(),
            false,
            tpl[0].clone(),
            &jong_all,
            &no_jong,
        ),
        build_selector(
            sel[1].clone(),
            "받침X/가로",
            h_jung.clone(),
            false,
            tpl[1].clone(),
            &jong_all,
            &no_jong,
        ),
        build_selector(sel[2].clone(), "받침O/세로", v_jung, true, tpl[2].clone(), &jong_all, &no_jong),
        build_selector(sel[3].clone(), "받침O/가로", h_jung, true, tpl[3].clone(), &jong_all, &no_jong),
    ];
    sys
}
