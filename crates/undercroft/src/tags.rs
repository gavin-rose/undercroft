use std::collections::BTreeMap;

use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

/// An interned terrain tag. Look one up by name with [`TagSet::get`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Tag(pub u16);

/// How a tag behaves and looks. Recipes can add or override tags under `[tags.<name>]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagDef {
    pub glyph: char,
    #[serde(default)]
    pub walkable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

/// The tags available to a world. Tags are semantic (`wall`, `grass`), so
/// renderers can map them to glyphs or sprites without knowing the style.
#[derive(Clone, Debug, PartialEq)]
pub struct TagSet {
    names: Vec<String>,
    defs: Vec<TagDef>,
    lookup: BTreeMap<String, Tag>,
}

/// `void` must stay first: new worlds are filled with `Tag(0)`.
const BUILTIN: &[(&str, char, bool, [u8; 3])] = &[
    ("void", ' ', false, [0, 0, 0]),
    ("wall", '#', false, [120, 112, 104]),
    ("floor", '.', true, [176, 164, 140]),
    ("stairs_up", '<', true, [240, 220, 120]),
    ("stairs_down", '>', true, [240, 220, 120]),
    ("deep_water", '~', false, [32, 64, 140]),
    ("water", '~', false, [60, 112, 200]),
    ("sand", '.', true, [222, 202, 140]),
    ("grass", '"', true, [100, 168, 72]),
    ("forest", 'T', true, [40, 112, 56]),
    ("hill", 'n', true, [150, 136, 84]),
    ("mountain", '^', false, [136, 126, 120]),
    ("snow", '^', false, [236, 236, 244]),
    ("town", 'o', true, [236, 120, 64]),
];

impl TagSet {
    pub fn builtin() -> Self {
        let mut set = TagSet {
            names: Vec::new(),
            defs: Vec::new(),
            lookup: BTreeMap::new(),
        };
        for &(name, glyph, walkable, color) in BUILTIN {
            set.define(
                name,
                TagDef {
                    glyph,
                    walkable,
                    color: Some(color),
                },
            );
        }
        set
    }

    /// Adds a tag, or replaces the definition of an existing one.
    pub fn define(&mut self, name: &str, def: TagDef) -> Tag {
        if let Some(&tag) = self.lookup.get(name) {
            self.defs[tag.0 as usize] = def;
            return tag;
        }
        let tag = Tag(u16::try_from(self.names.len()).expect("too many tags"));
        self.names.push(name.to_owned());
        self.defs.push(def);
        self.lookup.insert(name.to_owned(), tag);
        tag
    }

    pub fn get(&self, name: &str) -> Option<Tag> {
        self.lookup.get(name).copied()
    }

    pub fn name(&self, tag: Tag) -> &str {
        &self.names[tag.0 as usize]
    }

    pub fn def(&self, tag: Tag) -> &TagDef {
        &self.defs[tag.0 as usize]
    }
}

/// Serialized as a list indexed by tag id, so `terrain` cells can refer to it.
impl Serialize for TagSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Entry<'a> {
            name: &'a str,
            #[serde(flatten)]
            def: &'a TagDef,
        }
        let mut seq = serializer.serialize_seq(Some(self.names.len()))?;
        for (name, def) in self.names.iter().zip(&self.defs) {
            seq.serialize_element(&Entry { name, def })?;
        }
        seq.end()
    }
}
