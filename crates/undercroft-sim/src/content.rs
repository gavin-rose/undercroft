/// Sprites are `[column, row]` in the bundled Kenney 1-Bit Pack sheet.
#[derive(Debug)]
pub struct MonsterKind {
    pub name: &'static str,
    pub sprite: [u32; 2],
    pub color: [u8; 3],
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub sight: i32,
    pub min_depth: u32,
    /// Acts every other turn.
    pub slow: bool,
    /// Half its moves are random.
    pub erratic: bool,
}

const fn monster(
    name: &'static str,
    sprite: [u32; 2],
    color: [u8; 3],
    [hp, atk, def, sight]: [i32; 4],
    min_depth: u32,
) -> MonsterKind {
    MonsterKind {
        name,
        sprite,
        color,
        hp,
        atk,
        def,
        sight,
        min_depth,
        slow: false,
        erratic: false,
    }
}

pub const MONSTERS: &[MonsterKind] = &[
    monster("rat", [31, 7], [176, 146, 116], [3, 1, 0, 6], 1),
    MonsterKind {
        erratic: true,
        ..monster("bat", [26, 8], [160, 128, 200], [2, 1, 0, 8], 1)
    },
    MonsterKind {
        slow: true,
        ..monster("slime", [27, 8], [124, 214, 92], [7, 2, 0, 4], 2)
    },
    monster("snake", [28, 8], [112, 196, 128], [5, 3, 0, 6], 3),
    monster("goblin", [29, 2], [206, 176, 84], [8, 3, 1, 7], 4),
    monster("skeleton", [29, 6], [228, 226, 212], [10, 4, 1, 7], 5),
    monster("ghost", [27, 6], [168, 204, 255], [9, 4, 2, 9], 6),
    MonsterKind {
        slow: true,
        ..monster("troll", [30, 8], [184, 112, 92], [20, 6, 2, 6], 8)
    },
    monster("horror", [25, 8], [224, 92, 164], [24, 8, 3, 8], 10),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Potion,
    Gold(u32),
    Sword,
    Shield,
}

impl ItemKind {
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Potion => "potion",
            ItemKind::Gold(_) => "gold",
            ItemKind::Sword => "sword",
            ItemKind::Shield => "shield",
        }
    }

    pub fn sprite(self) -> [u32; 2] {
        match self {
            ItemKind::Potion => [32, 13],
            ItemKind::Gold(_) => [41, 3],
            ItemKind::Sword => [32, 8],
            ItemKind::Shield => [37, 3],
        }
    }

    pub fn color(self) -> [u8; 3] {
        match self {
            ItemKind::Potion => [255, 96, 120],
            ItemKind::Gold(_) => [255, 210, 64],
            ItemKind::Sword => [200, 220, 240],
            ItemKind::Shield => [120, 180, 255],
        }
    }
}
