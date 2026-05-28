pub mod core;
pub mod io;

pub use crate::core::engine::{FallbackEntry, LayoutEngine, LayoutResult, PartPlacement, PlacedPart};
pub use crate::core::glyph::{GlyphKey, GlyphStore, PixelGlyph};
pub use crate::core::groups::ComponentGroup;
pub use crate::core::old_hangul::{apply_old_hangul_rules, default_archaic_map, jamo_matches_kind};
pub use crate::core::rules::{CharSetCond, GroupRef, RuleSystem, SelectorRule, Template, VariantRule};
pub use crate::core::types::HangulComponent;
pub use crate::io::project::{ProjectData, parse_project_bytes, serialize_project_to_yaml};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::io::project::{load_project_from_path, save_project_to_path};
