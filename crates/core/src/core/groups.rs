use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::core::types::HangulComponent;

mod char_set_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::BTreeSet;

    pub fn serialize<S: Serializer>(set: &BTreeSet<char>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&set.iter().collect::<String>())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeSet<char>, D::Error> {
        Ok(String::deserialize(d)?.chars().collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentGroup {
    pub id: String,
    pub name: String,
    pub target: HangulComponent,
    #[serde(with = "char_set_serde")]
    pub members: BTreeSet<char>,
}

pub fn group_display_name(groups: &[ComponentGroup], id: &str) -> String {
    groups
        .iter()
        .find(|g| g.id == id)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| format!("(missing) {}", id))
}
