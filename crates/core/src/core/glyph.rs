use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::core::types::HangulComponent;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlyphKey {
    pub kind: HangulComponent,
    pub jamo: char,
    pub group_id: String,
}

impl GlyphKey {
    pub fn new(kind: HangulComponent, jamo: char, group_id: impl Into<String>) -> Self {
        Self {
            kind,
            jamo,
            group_id: group_id.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PixelGlyph {
    pub pixels: BTreeSet<(i32, i32)>,
}

impl PixelGlyph {
    pub fn has(&self, x: i32, y: i32) -> bool {
        self.pixels.contains(&(x, y))
    }

    pub fn set(&mut self, x: i32, y: i32) {
        self.pixels.insert((x, y));
    }

    pub fn clear(&mut self, x: i32, y: i32) {
        self.pixels.remove(&(x, y));
    }

    pub fn shift(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        self.pixels = self.pixels.iter().map(|(x, y)| (x + dx, y + dy)).collect();
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct GlyphStore {
    pub glyphs: BTreeMap<GlyphKey, PixelGlyph>,
}

impl GlyphStore {
    pub fn get(&self, key: &GlyphKey) -> Option<&PixelGlyph> {
        self.glyphs.get(key)
    }

    pub fn get_mut(&mut self, key: &GlyphKey) -> Option<&mut PixelGlyph> {
        self.glyphs.get_mut(key)
    }

    pub fn has(&self, key: &GlyphKey) -> bool {
        self.glyphs.contains_key(key)
    }

    pub fn ensure_glyph(&mut self, key: GlyphKey) {
        self.glyphs.entry(key).or_default();
    }

    pub fn remove_group_glyphs(&mut self, group_id: &str) {
        self.glyphs.retain(|k, _| k.group_id != group_id);
    }

    pub fn clone_group_glyphs(&mut self, from_gid: &str, to_gid: &str) {
        if from_gid == to_gid {
            return;
        }

        let clones: Vec<(GlyphKey, PixelGlyph)> = self
            .glyphs
            .iter()
            .filter_map(|(key, glyph)| {
                if key.group_id == from_gid {
                    Some((GlyphKey::new(key.kind, key.jamo, to_gid), glyph.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (k, g) in clones {
            self.glyphs.insert(k, g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::HangulComponent;

    #[test]
    fn pixel_glyph_new_empty() {
        assert!(PixelGlyph::default().pixels.is_empty());
    }

    #[test]
    fn pixel_glyph_has_returns_true_after_set() {
        let mut g = PixelGlyph::default();
        g.set(3, 5);
        assert!(g.has(3, 5));
    }

    #[test]
    fn pixel_glyph_has_returns_false_unset() {
        let g = PixelGlyph::default();
        assert!(!g.has(0, 0));
    }

    #[test]
    fn pixel_glyph_set_and_check() {
        let mut g = PixelGlyph::default();
        g.set(3, 5);
        assert!(g.has(3, 5));
    }

    #[test]
    fn pixel_glyph_set_duplicate() {
        let mut g = PixelGlyph::default();
        g.set(1, 1);
        g.set(1, 1);
        assert_eq!(g.pixels.len(), 1);
    }

    #[test]
    fn pixel_glyph_clear() {
        let mut g = PixelGlyph::default();
        g.set(2, 2);
        g.clear(2, 2);
        assert!(!g.has(2, 2));
    }

    #[test]
    fn pixel_glyph_clear_nonexistent() {
        let mut g = PixelGlyph::default();
        g.clear(0, 0);
        assert!(g.pixels.is_empty());
    }

    #[test]
    fn pixel_glyph_shift_zero_noop() {
        let mut g = PixelGlyph::default();
        g.set(1, 1);
        g.shift(0, 0);
        assert!(g.pixels.contains(&(1, 1)));
    }

    #[test]
    fn pixel_glyph_shift_positive() {
        let mut g = PixelGlyph::default();
        g.set(0, 0);
        g.shift(3, 4);
        assert!(g.pixels.contains(&(3, 4)));
        assert!(!g.pixels.contains(&(0, 0)));
    }

    #[test]
    fn pixel_glyph_shift_negative() {
        let mut g = PixelGlyph::default();
        g.set(5, 5);
        g.shift(-2, -3);
        assert!(g.pixels.contains(&(3, 2)));
        assert!(!g.pixels.contains(&(5, 5)));
    }

    #[test]
    fn pixel_glyph_shift_preserves_count() {
        let mut g = PixelGlyph::default();
        g.set(0, 0);
        g.set(1, 0);
        g.set(2, 0);
        g.shift(1, 1);
        assert_eq!(g.pixels.len(), 3);
    }

    #[test]
    fn store_has_false_when_empty() {
        assert!(!GlyphStore::default().has(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1")));
    }

    #[test]
    fn store_ensure_creates_entry() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        assert!(store.has(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1")));
    }

    #[test]
    fn store_ensure_idempotent() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        assert_eq!(store.glyphs.len(), 1);
    }

    #[test]
    fn store_get_returns_none_missing() {
        let store = GlyphStore::default();
        assert!(store.get(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1")).is_none());
    }

    #[test]
    fn store_get_returns_some_after_ensure() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        assert!(store.get(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1")).is_some());
    }

    #[test]
    fn store_get_mut_allows_mutation() {
        let mut store = GlyphStore::default();
        let key = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1");
        store.ensure_glyph(key.clone());
        store.get_mut(&key).unwrap().set(1, 1);
        assert_eq!(store.get(&key).unwrap().pixels.len(), 1);
    }

    #[test]
    fn store_remove_group_removes_only_that_group() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g2"));
        store.remove_group_glyphs("g1");
        assert!(!store.has(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1")));
        assert!(store.has(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g2")));
    }

    #[test]
    fn store_clone_group_glyphs_copies_pixels() {
        let mut store = GlyphStore::default();
        let key1 = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1");
        store.ensure_glyph(key1.clone());
        store.get_mut(&key1).unwrap().set(5, 5);
        store.clone_group_glyphs("g1", "g2");
        let key2 = GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g2");
        assert!(store.get(&key2).unwrap().pixels.contains(&(5, 5)));
    }

    #[test]
    fn store_clone_group_same_id_noop() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g1"));
        store.clone_group_glyphs("g1", "g1");
        assert_eq!(store.glyphs.len(), 1);
    }
}
