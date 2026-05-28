use crate::core::engine::{LayoutEngine, LayoutResult, PartPlacement};
use crate::core::glyph::{GlyphKey, GlyphStore};

pub struct RenderContext<'a> {
    pub store: &'a GlyphStore,
    pub engine: &'a LayoutEngine,
    pub canvas_w: i32,
    pub canvas_h: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ComposedPixel {
    pub x: i32,
    pub y: i32,
}

fn push_part_pixels(out: &mut Vec<ComposedPixel>, store: &GlyphStore, p: &PartPlacement, group_id: &str, canvas_w: i32, canvas_h: i32) {
    let key = GlyphKey::new(p.kind, p.jamo, group_id);
    let Some(g) = store.get(&key) else {
        return;
    };

    for &(x, y) in &g.pixels {
        if x < 0 || y < 0 || x >= canvas_w || y >= canvas_h {
            continue;
        }
        out.push(ComposedPixel { x, y });
    }
}

pub fn compose_pixels(store: &GlyphStore, layout: &LayoutResult, canvas_w: i32, canvas_h: i32) -> Vec<ComposedPixel> {
    let mut out = Vec::new();

    push_part_pixels(&mut out, store, &layout.cho.placement, &layout.cho.group_id, canvas_w, canvas_h);
    push_part_pixels(&mut out, store, &layout.jung.placement, &layout.jung.group_id, canvas_w, canvas_h);
    if let Some(jong) = &layout.jong {
        push_part_pixels(&mut out, store, &jong.placement, &jong.group_id, canvas_w, canvas_h);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::{LayoutResult, PartPlacement, PlacedPart};
    use crate::core::glyph::GlyphStore;
    use crate::core::types::HangulComponent;

    fn make_layout(
        cho: (HangulComponent, char, &str),
        jung: (HangulComponent, char, &str),
        jong: Option<(HangulComponent, char, &str)>,
    ) -> LayoutResult {
        LayoutResult {
            template_id: "tpl".to_string(),
            template_name: "tpl".to_string(),
            selector_name: "sel".to_string(),
            matched_variants: vec![],
            cho: PlacedPart {
                placement: PartPlacement { kind: cho.0, jamo: cho.1 },
                group_id: cho.2.to_string(),
            },
            jung: PlacedPart {
                placement: PartPlacement {
                    kind: jung.0,
                    jamo: jung.1,
                },
                group_id: jung.2.to_string(),
            },
            jong: jong.map(|(k, j, g)| PlacedPart {
                placement: PartPlacement { kind: k, jamo: j },
                group_id: g.to_string(),
            }),
            fallbacks: vec![],
        }
    }

    fn store_with_pixel(kind: HangulComponent, jamo: char, gid: &str, x: i32, y: i32) -> GlyphStore {
        let mut store = GlyphStore::default();
        let key = GlyphKey::new(kind, jamo, gid);
        store.ensure_glyph(key.clone());
        store.get_mut(&key).unwrap().set(x, y);
        store
    }

    #[test]
    fn compose_empty_store_returns_empty() {
        let store = GlyphStore::default();
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        assert!(compose_pixels(&store, &layout, 16, 16).is_empty());
    }

    #[test]
    fn compose_cho_and_jung_both_appear() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Jung, 'ㅏ', "g"));
        store.get_mut(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g")).unwrap().set(0, 0);
        store.get_mut(&GlyphKey::new(HangulComponent::Jung, 'ㅏ', "g")).unwrap().set(1, 1);

        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        let out = compose_pixels(&store, &layout, 16, 16);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|p| p.x == 0 && p.y == 0));
        assert!(out.iter().any(|p| p.x == 1 && p.y == 1));
    }

    #[test]
    fn compose_jong_included_when_in_layout() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Jung, 'ㅏ', "g"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Jong, 'ㄴ', "g"));
        store.get_mut(&GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g")).unwrap().set(0, 0);
        store.get_mut(&GlyphKey::new(HangulComponent::Jung, 'ㅏ', "g")).unwrap().set(1, 1);
        store.get_mut(&GlyphKey::new(HangulComponent::Jong, 'ㄴ', "g")).unwrap().set(2, 2);

        let layout = make_layout(
            (HangulComponent::Cho, 'ㄱ', "g"),
            (HangulComponent::Jung, 'ㅏ', "g"),
            Some((HangulComponent::Jong, 'ㄴ', "g")),
        );
        assert_eq!(compose_pixels(&store, &layout, 16, 16).len(), 3);
    }

    #[test]
    fn compose_clips_negative_x() {
        let store = store_with_pixel(HangulComponent::Cho, 'ㄱ', "g", -1, 0);
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        assert!(compose_pixels(&store, &layout, 16, 16).is_empty());
    }

    #[test]
    fn compose_clips_negative_y() {
        let store = store_with_pixel(HangulComponent::Cho, 'ㄱ', "g", 0, -1);
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        assert!(compose_pixels(&store, &layout, 16, 16).is_empty());
    }

    #[test]
    fn compose_clips_x_equal_canvas_w() {
        let store = store_with_pixel(HangulComponent::Cho, 'ㄱ', "g", 16, 0);
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        assert!(compose_pixels(&store, &layout, 16, 16).is_empty());
    }

    #[test]
    fn compose_clips_y_equal_canvas_h() {
        let store = store_with_pixel(HangulComponent::Cho, 'ㄱ', "g", 0, 16);
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        assert!(compose_pixels(&store, &layout, 16, 16).is_empty());
    }

    #[test]
    fn compose_includes_last_valid_pixel() {
        let store = store_with_pixel(HangulComponent::Cho, 'ㄱ', "g", 15, 15);
        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        let out = compose_pixels(&store, &layout, 16, 16);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].x, 15);
        assert_eq!(out[0].y, 15);
    }

    #[test]
    fn compose_jong_absent_in_layout_not_rendered() {
        let mut store = GlyphStore::default();
        store.ensure_glyph(GlyphKey::new(HangulComponent::Cho, 'ㄱ', "g"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Jung, 'ㅏ', "g"));
        store.ensure_glyph(GlyphKey::new(HangulComponent::Jong, 'ㄴ', "g"));
        store.get_mut(&GlyphKey::new(HangulComponent::Jong, 'ㄴ', "g")).unwrap().set(5, 5);

        let layout = make_layout((HangulComponent::Cho, 'ㄱ', "g"), (HangulComponent::Jung, 'ㅏ', "g"), None);
        let out = compose_pixels(&store, &layout, 16, 16);
        assert!(!out.iter().any(|p| p.x == 5 && p.y == 5));
    }
}
