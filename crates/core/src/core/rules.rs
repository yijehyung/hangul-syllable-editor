use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::core::{
    glyph::{GlyphKey, GlyphStore},
    groups::ComponentGroup,
    types::HangulComponent,
};

#[derive(Clone, Debug)]
pub struct GroupRef {
    pub template_name: String,
    /// `None` = default group slot, `Some` = variant rule name
    pub rule_name: Option<String>,
    pub component: HangulComponent,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CharSetCond {
    #[default]
    Any,
    Include(BTreeSet<char>),
    Exclude(BTreeSet<char>),
}

impl Serialize for CharSetCond {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        enum Proxy {
            Any,
            Include(String),
            Exclude(String),
        }
        let proxy = match self {
            CharSetCond::Any => Proxy::Any,
            CharSetCond::Include(set) => Proxy::Include(set.iter().collect()),
            CharSetCond::Exclude(set) => Proxy::Exclude(set.iter().collect()),
        };
        proxy.serialize(s)
    }
}

impl<'de> Deserialize<'de> for CharSetCond {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        enum Proxy {
            Any,
            Include(String),
            Exclude(String),
        }
        match Proxy::deserialize(d)? {
            Proxy::Any => Ok(CharSetCond::Any),
            Proxy::Include(s) => Ok(CharSetCond::Include(s.chars().collect())),
            Proxy::Exclude(s) => Ok(CharSetCond::Exclude(s.chars().collect())),
        }
    }
}

impl CharSetCond {
    pub fn matches(&self, c: char) -> bool {
        match self {
            CharSetCond::Any => true,
            CharSetCond::Include(set) => set.contains(&c),
            CharSetCond::Exclude(set) => !set.contains(&c),
        }
    }

    pub fn implies_some_constraint(&self) -> bool {
        !matches!(self, CharSetCond::Any)
    }

    pub fn toggle(&mut self, c: char) {
        match self {
            CharSetCond::Any => {
                let mut s = BTreeSet::new();
                s.insert(c);
                *self = CharSetCond::Include(s);
            }
            CharSetCond::Include(set) | CharSetCond::Exclude(set) => {
                if set.contains(&c) {
                    set.remove(&c);
                } else {
                    set.insert(c);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        match self {
            CharSetCond::Any => {}
            CharSetCond::Include(_) | CharSetCond::Exclude(_) => *self = CharSetCond::Any,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariantRule {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub priority: i32,
    pub cho_set: CharSetCond,
    pub jung_set: CharSetCond,
    pub jong_set: CharSetCond,

    #[serde(default)]
    pub set_cho_group_id: Option<String>,
    #[serde(default)]
    pub set_jung_group_id: Option<String>,
    #[serde(default)]
    pub set_jong_group_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    #[serde(default)]
    pub name: String,

    pub default_cho_group_id: String,
    pub default_jung_group_id: String,
    #[serde(default)]
    pub default_jong_group_id: Option<String>,

    pub variant_rules: Vec<VariantRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectorRule {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub priority: i32,
    pub cho_set: CharSetCond,
    pub jung_set: CharSetCond,
    pub jong_set: CharSetCond,
    pub template_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RuleSystem {
    pub groups: Vec<ComponentGroup>,
    pub templates: Vec<Template>,
    pub selectors: Vec<SelectorRule>,

    pub base_cho_group_id: String,
    pub base_jung_group_id: String,
    pub base_jong_group_id: String,

    next_group_seq: u64,
    #[serde(default)]
    next_template_seq: u64,
    #[serde(default)]
    next_selector_seq: u64,
    #[serde(default)]
    next_variant_seq: u64,
}

impl RuleSystem {
    pub fn new_group_id(&mut self) -> String {
        let id = format!("grp_{:06}", self.next_group_seq);
        self.next_group_seq += 1;
        id
    }

    pub fn new_template_id(&mut self) -> String {
        let id = format!("tpl_{:06}", self.next_template_seq);
        self.next_template_seq += 1;
        id
    }

    pub fn new_selector_id(&mut self) -> String {
        let id = format!("sel_{:06}", self.next_selector_seq);
        self.next_selector_seq += 1;
        id
    }

    pub fn new_variant_id(&mut self) -> String {
        let id = format!("vr_{:06}", self.next_variant_seq);
        self.next_variant_seq += 1;
        id
    }

    pub fn find_group(&self, id: &str) -> Option<&ComponentGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn find_group_mut(&mut self, id: &str) -> Option<&mut ComponentGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    pub fn add_group(&mut self, store: &mut GlyphStore, name: &str, target: HangulComponent, members: BTreeSet<char>) -> String {
        let id = self.new_group_id();
        self.groups.push(ComponentGroup {
            id: id.clone(),
            name: name.into(),
            target,
            members,
        });
        self.ensure_group_glyphs(store, &id);
        id
    }

    pub fn remove_group(&mut self, store: &mut GlyphStore, id: &str) {
        self.groups.retain(|g| g.id != id);
        store.remove_group_glyphs(id);
    }

    pub fn rename_group(&mut self, id: &str, new_name: &str) {
        if let Some(g) = self.find_group_mut(id) {
            g.name = new_name.to_string();
        }
    }

    pub fn move_group_up(&mut self, id: &str) {
        if let Some(idx) = self.groups.iter().position(|g| g.id == id)
            && idx > 0
        {
            self.groups.swap(idx, idx - 1);
        }
    }

    pub fn move_group_down(&mut self, id: &str) {
        if let Some(idx) = self.groups.iter().position(|g| g.id == id)
            && idx + 1 < self.groups.len()
        {
            self.groups.swap(idx, idx + 1);
        }
    }

    pub fn collect_group_refs(&self, gid: &str) -> Vec<GroupRef> {
        let mut refs = Vec::new();
        for t in &self.templates {
            let tname = if t.name.is_empty() { t.id.as_str() } else { t.name.as_str() };
            if t.default_cho_group_id == gid {
                refs.push(GroupRef {
                    template_name: tname.into(),
                    rule_name: None,
                    component: HangulComponent::Cho,
                });
            }
            if t.default_jung_group_id == gid {
                refs.push(GroupRef {
                    template_name: tname.into(),
                    rule_name: None,
                    component: HangulComponent::Jung,
                });
            }
            if t.default_jong_group_id.as_deref() == Some(gid) {
                refs.push(GroupRef {
                    template_name: tname.into(),
                    rule_name: None,
                    component: HangulComponent::Jong,
                });
            }
            for r in &t.variant_rules {
                let rname = if r.name.is_empty() { r.id.as_str() } else { r.name.as_str() };
                if r.set_cho_group_id.as_deref() == Some(gid) {
                    refs.push(GroupRef {
                        template_name: tname.into(),
                        rule_name: Some(rname.into()),
                        component: HangulComponent::Cho,
                    });
                }
                if r.set_jung_group_id.as_deref() == Some(gid) {
                    refs.push(GroupRef {
                        template_name: tname.into(),
                        rule_name: Some(rname.into()),
                        component: HangulComponent::Jung,
                    });
                }
                if r.set_jong_group_id.as_deref() == Some(gid) {
                    refs.push(GroupRef {
                        template_name: tname.into(),
                        rule_name: Some(rname.into()),
                        component: HangulComponent::Jong,
                    });
                }
            }
        }
        refs
    }

    pub fn is_group_referenced(&self, gid: &str) -> bool {
        for t in &self.templates {
            if t.default_cho_group_id == gid || t.default_jung_group_id == gid || t.default_jong_group_id.as_deref() == Some(gid) {
                return true;
            }
            for r in &t.variant_rules {
                if r.set_cho_group_id.as_deref() == Some(gid)
                    || r.set_jung_group_id.as_deref() == Some(gid)
                    || r.set_jong_group_id.as_deref() == Some(gid)
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn base_group_id_for_kind(&self, kind: HangulComponent) -> &str {
        match kind {
            HangulComponent::Cho => &self.base_cho_group_id,
            HangulComponent::Jung => &self.base_jung_group_id,
            HangulComponent::Jong => &self.base_jong_group_id,
        }
    }

    pub fn ensure_group_glyphs(&self, store: &mut GlyphStore, group_id: &str) {
        let Some(g) = self.find_group(group_id) else { return };
        for &jamo in &g.members {
            store.ensure_glyph(GlyphKey::new(g.target, jamo, &g.id));
        }
    }

    pub fn find_template(&self, id: &str) -> Option<&Template> {
        self.templates.iter().find(|t| t.id == id)
    }

    pub fn find_template_mut(&mut self, id: &str) -> Option<&mut Template> {
        self.templates.iter_mut().find(|t| t.id == id)
    }

    pub fn add_template(&mut self, tpl: Template) {
        self.templates.push(tpl);
    }

    pub fn remove_template(&mut self, id: &str) {
        self.templates.retain(|t| t.id != id);
    }

    pub fn move_template_up(&mut self, id: &str) {
        if let Some(idx) = self.templates.iter().position(|t| t.id == id)
            && idx > 0
        {
            self.templates.swap(idx, idx - 1);
        }
    }

    pub fn move_template_down(&mut self, id: &str) {
        if let Some(idx) = self.templates.iter().position(|t| t.id == id)
            && idx + 1 < self.templates.len()
        {
            self.templates.swap(idx, idx + 1);
        }
    }

    pub fn is_template_referenced(&self, tid: &str) -> bool {
        self.selectors.iter().any(|s| s.template_id == tid)
    }

    pub fn find_selector(&self, id: &str) -> Option<&SelectorRule> {
        self.selectors.iter().find(|s| s.id == id)
    }

    pub fn find_selector_mut(&mut self, id: &str) -> Option<&mut SelectorRule> {
        self.selectors.iter_mut().find(|s| s.id == id)
    }

    pub fn add_selector(&mut self, sel: SelectorRule) {
        self.selectors.push(sel);
    }

    pub fn remove_selector(&mut self, id: &str) {
        self.selectors.retain(|s| s.id != id);
    }

    pub fn move_selector_up(&mut self, id: &str) {
        if let Some(idx) = self.selectors.iter().position(|s| s.id == id)
            && idx > 0
        {
            self.selectors.swap(idx, idx - 1);
        }
    }

    pub fn move_selector_down(&mut self, id: &str) {
        if let Some(idx) = self.selectors.iter().position(|s| s.id == id)
            && idx + 1 < self.selectors.len()
        {
            self.selectors.swap(idx, idx + 1);
        }
    }

    pub(crate) fn reset_seqs(&mut self, groups: u64, templates: u64, selectors: u64, variants: u64) {
        self.next_group_seq = groups;
        self.next_template_seq = templates;
        self.next_selector_seq = selectors;
        self.next_variant_seq = variants;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::groups::{ComponentGroup, group_display_name};
    use crate::core::types::HangulComponent;

    fn include(chars: &[char]) -> CharSetCond {
        CharSetCond::Include(chars.iter().copied().collect())
    }

    fn exclude(chars: &[char]) -> CharSetCond {
        CharSetCond::Exclude(chars.iter().copied().collect())
    }

    #[test]
    fn any_matches_any_char() {
        assert!(CharSetCond::Any.matches('a'));
        assert!(CharSetCond::Any.matches('\0'));
    }

    #[test]
    fn include_matches_member() {
        assert!(include(&['ㄱ', 'ㄴ']).matches('ㄱ'));
    }

    #[test]
    fn include_no_match_non_member() {
        assert!(!include(&['ㄱ', 'ㄴ']).matches('ㄷ'));
    }

    #[test]
    fn include_empty_never_matches() {
        assert!(!include(&[]).matches('ㄱ'));
    }

    #[test]
    fn exclude_matches_non_member() {
        assert!(exclude(&['ㄱ']).matches('ㄴ'));
    }

    #[test]
    fn exclude_no_match_member() {
        assert!(!exclude(&['ㄱ']).matches('ㄱ'));
    }

    #[test]
    fn exclude_empty_matches_everything() {
        assert!(exclude(&[]).matches('ㄱ'));
    }

    #[test]
    fn any_has_no_constraint() {
        assert!(!CharSetCond::Any.implies_some_constraint());
    }

    #[test]
    fn include_nonempty_has_constraint() {
        assert!(include(&['ㄱ']).implies_some_constraint());
    }

    #[test]
    fn exclude_nonempty_has_constraint() {
        assert!(exclude(&['ㄱ']).implies_some_constraint());
    }

    #[test]
    fn include_empty_still_has_constraint() {
        assert!(include(&[]).implies_some_constraint());
    }

    #[test]
    fn toggle_any_becomes_include_single() {
        let mut cond = CharSetCond::Any;
        cond.toggle('ㄱ');
        assert_eq!(cond, include(&['ㄱ']));
    }

    #[test]
    fn toggle_include_adds_new() {
        let mut cond = include(&['ㄱ']);
        cond.toggle('ㄴ');
        assert!(matches!(&cond, CharSetCond::Include(s) if s.contains(&'ㄱ') && s.contains(&'ㄴ')));
    }

    #[test]
    fn toggle_include_removes_existing() {
        let mut cond = include(&['ㄱ', 'ㄴ']);
        cond.toggle('ㄱ');
        assert!(matches!(&cond, CharSetCond::Include(s) if !s.contains(&'ㄱ') && s.contains(&'ㄴ')));
    }

    #[test]
    fn clear_include_resets_to_any() {
        let mut cond = include(&['ㄱ', 'ㄴ']);
        cond.clear();
        assert_eq!(cond, CharSetCond::Any);
        assert!(cond.matches('ㄱ'), "clear 후 Any이므로 모든 자모 매칭되어야 함");
    }

    #[test]
    fn clear_exclude_resets_to_any() {
        let mut cond = exclude(&['ㄱ']);
        cond.clear();
        assert_eq!(cond, CharSetCond::Any);
    }

    #[test]
    fn clear_any_stays_any() {
        let mut cond = CharSetCond::Any;
        cond.clear();
        assert_eq!(cond, CharSetCond::Any);
    }

    #[test]
    fn toggle_exclude_adds_new() {
        let mut cond = exclude(&['ㄱ']);
        cond.toggle('ㄴ');
        assert!(matches!(&cond, CharSetCond::Exclude(s) if s.contains(&'ㄴ')));
    }

    #[test]
    fn new_group_id_increments() {
        let mut rs = RuleSystem::default();
        let a = rs.new_group_id();
        let b = rs.new_group_id();
        assert_eq!(a, "grp_000000");
        assert_eq!(b, "grp_000001");
    }

    #[test]
    fn new_variant_id_format() {
        let mut rs = RuleSystem::default();
        assert_eq!(rs.new_variant_id(), "vr_000000");
    }

    #[test]
    fn id_counters_are_independent() {
        let mut rs = RuleSystem::default();
        rs.new_group_id();
        rs.new_group_id();
        rs.new_template_id();
        assert_eq!(rs.new_group_id(), "grp_000002");
        assert_eq!(rs.new_template_id(), "tpl_000001");
        assert_eq!(rs.new_selector_id(), "sel_000000");
    }

    #[test]
    fn group_display_name_found() {
        let groups = vec![ComponentGroup {
            id: "g1".to_string(),
            name: "테스트".to_string(),
            target: HangulComponent::Cho,
            members: Default::default(),
        }];
        assert_eq!(group_display_name(&groups, "g1"), "테스트");
    }

    #[test]
    fn group_display_name_missing() {
        let groups: Vec<ComponentGroup> = vec![];
        assert_eq!(group_display_name(&groups, "g1"), "(missing) g1");
    }
}
