use serde::{Deserialize, Serialize};
use undercroft::{Pos, Recipe, Rng, Seed, World, generate};

use crate::content::{ItemKind, MONSTERS, MonsterKind};
use crate::fov;

pub const FLOORS: u32 = 10;
const SIGHT: i32 = 8;
const LOG_KEEP: usize = 6;
const REGEN_EVERY: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Wait,
    Descend,
    Quaff,
}

impl Action {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "up" => Action::Up,
            "down" => Action::Down,
            "left" => Action::Left,
            "right" => Action::Right,
            "wait" => Action::Wait,
            "descend" => Action::Descend,
            "quaff" => Action::Quaff,
            _ => return None,
        })
    }

    fn code(self) -> char {
        match self {
            Action::Up => 'u',
            Action::Down => 'd',
            Action::Left => 'l',
            Action::Right => 'r',
            Action::Wait => 'w',
            Action::Descend => 'x',
            Action::Quaff => 'q',
        }
    }

    fn from_code(c: char) -> Option<Self> {
        Some(match c {
            'u' => Action::Up,
            'd' => Action::Down,
            'l' => Action::Left,
            'r' => Action::Right,
            'w' => Action::Wait,
            'x' => Action::Descend,
            'q' => Action::Quaff,
            _ => return None,
        })
    }

    fn delta(self) -> Option<(i32, i32)> {
        match self {
            Action::Up => Some((0, -1)),
            Action::Down => Some((0, 1)),
            Action::Left => Some((-1, 0)),
            Action::Right => Some((1, 0)),
            _ => None,
        }
    }
}

/// Run-length encodes actions, e.g. `rrrru` becomes `r4u`.
pub fn encode(actions: &[Action]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < actions.len() {
        let run = actions[i..]
            .iter()
            .take_while(|&&a| a == actions[i])
            .count();
        out.push(actions[i].code());
        if run > 1 {
            out.push_str(&run.to_string());
        }
        i += run;
    }
    out
}

pub fn decode(code: &str) -> Option<Vec<Action>> {
    let mut actions = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        let action = Action::from_code(c)?;
        let mut digits = String::new();
        while let Some(d) = chars.next_if(char::is_ascii_digit) {
            digits.push(d);
        }
        let run = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        actions.extend(std::iter::repeat_n(action, run));
    }
    Some(actions)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Playing,
    Dead,
    Won,
}

#[derive(Clone, Debug)]
struct Player {
    pos: Pos,
    hp: i32,
    max_hp: i32,
    atk: i32,
    def: i32,
}

#[derive(Clone, Debug)]
struct Monster {
    kind: usize,
    pos: Pos,
    hp: i32,
    awake: bool,
}

#[derive(Clone, Debug)]
struct Item {
    kind: ItemKind,
    pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Game {
    seed: Seed,
    depth: u32,
    world: World,
    player: Player,
    monsters: Vec<Monster>,
    items: Vec<Item>,
    explored: Vec<bool>,
    visible: Vec<bool>,
    turn: u32,
    potions: u32,
    gold: u32,
    kills: u32,
    status: Status,
    /// Combat and AI rolls. Floors and spawns use their own seeds, so every
    /// player sees the same dungeon no matter how they play.
    rng: Rng,
    log: Vec<String>,
    actions: Vec<Action>,
    killed_by: Option<&'static str>,
}

/// Everything a frontend needs to draw the current state.
#[derive(Debug, Serialize)]
pub struct View<'a> {
    pub seed: String,
    pub depth: u32,
    pub floors: u32,
    pub turn: u32,
    pub status: Status,
    pub hp: i32,
    pub max_hp: i32,
    pub atk: i32,
    pub def: i32,
    pub potions: u32,
    pub gold: u32,
    pub kills: u32,
    pub score: u32,
    pub killed_by: Option<&'static str>,
    pub player: Pos,
    pub on_stairs: bool,
    pub width: i32,
    pub height: i32,
    /// One char per tile: `0` unseen, `1` remembered, `2` visible.
    pub fog: String,
    pub monsters: Vec<SpriteView>,
    pub items: Vec<SpriteView>,
    pub log: &'a [String],
}

#[derive(Debug, Serialize)]
pub struct SpriteView {
    pub name: &'static str,
    pub pos: Pos,
    pub sprite: [u32; 2],
    pub color: [u8; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp: Option<[i32; 2]>,
}

fn build_floor(seed: Seed, depth: u32) -> World {
    let style = if depth % 3 == 0 { "caves" } else { "dungeon" };
    let recipe = Recipe::style(style).expect("built-in style");
    generate(seed.derive(&format!("floor:{depth}")), &recipe)
        .expect("built-in styles always generate")
}

impl Game {
    pub fn new(seed: Seed) -> Self {
        let world = build_floor(seed, 1);
        let cells = world.terrain.cells().len();
        let mut game = Game {
            seed,
            depth: 1,
            player: Player {
                pos: world.marker("stairs_up").expect("floors have stairs"),
                hp: 20,
                max_hp: 20,
                atk: 3,
                def: 0,
            },
            world,
            monsters: Vec::new(),
            items: Vec::new(),
            explored: vec![false; cells],
            visible: vec![false; cells],
            turn: 0,
            potions: 1,
            gold: 0,
            kills: 0,
            status: Status::Playing,
            rng: Rng::new(seed.derive("combat").0),
            log: Vec::new(),
            actions: Vec::new(),
            killed_by: None,
        };
        game.populate();
        game.update_fov();
        game.say("You descend into the undercroft.");
        game
    }

    /// Rebuilds a run from its seed and recorded actions.
    pub fn replay(seed: Seed, actions: &[Action]) -> Self {
        let mut game = Game::new(seed);
        for &action in actions {
            game.act(action);
        }
        game
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn score(&self) -> u32 {
        let won = if self.status == Status::Won { 1000 } else { 0 };
        (self.depth - 1) * 100 + self.kills * 10 + self.gold + won
    }

    /// Applies one action. Returns false (and records nothing) if it didn't
    /// take a turn, like walking into a wall.
    pub fn act(&mut self, action: Action) -> bool {
        if self.status != Status::Playing {
            return false;
        }
        let took_turn = match action {
            Action::Wait => true,
            Action::Quaff => self.quaff(),
            Action::Descend => self.descend(),
            _ => self.step(action),
        };
        if !took_turn {
            return false;
        }
        self.actions.push(action);
        self.turn += 1;
        if action != Action::Descend {
            self.monsters_act();
        }
        if self.status == Status::Playing && self.turn % REGEN_EVERY == 0 {
            self.player.hp = (self.player.hp + 1).min(self.player.max_hp);
        }
        self.update_fov();
        true
    }

    pub fn view(&self) -> View<'_> {
        let fog = self
            .explored
            .iter()
            .zip(&self.visible)
            .map(|(&seen, &now)| match (seen, now) {
                (_, true) => '2',
                (true, false) => '1',
                _ => '0',
            })
            .collect();
        let shown = |pos: Pos| {
            self.world
                .terrain
                .index(pos)
                .is_some_and(|i| self.visible[i])
        };
        View {
            seed: self.seed.0.to_string(),
            depth: self.depth,
            floors: FLOORS,
            turn: self.turn,
            status: self.status,
            hp: self.player.hp,
            max_hp: self.player.max_hp,
            atk: self.player.atk,
            def: self.player.def,
            potions: self.potions,
            gold: self.gold,
            kills: self.kills,
            score: self.score(),
            killed_by: self.killed_by,
            player: self.player.pos,
            on_stairs: self.world.marker("stairs_down") == Some(self.player.pos),
            width: self.world.width(),
            height: self.world.height(),
            fog,
            monsters: self
                .monsters
                .iter()
                .filter(|m| shown(m.pos))
                .map(|m| {
                    let kind = &MONSTERS[m.kind];
                    SpriteView {
                        name: kind.name,
                        pos: m.pos,
                        sprite: kind.sprite,
                        color: kind.color,
                        hp: Some([m.hp, kind.hp]),
                    }
                })
                .collect(),
            items: self
                .items
                .iter()
                .filter(|it| shown(it.pos))
                .map(|it| SpriteView {
                    name: it.kind.name(),
                    pos: it.pos,
                    sprite: it.kind.sprite(),
                    color: it.kind.color(),
                    hp: None,
                })
                .collect(),
            log: &self.log[self.log.len().saturating_sub(LOG_KEEP)..],
        }
    }

    fn say(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > LOG_KEEP * 4 {
            self.log.drain(..LOG_KEEP * 2);
        }
    }

    fn enter_floor(&mut self, depth: u32) {
        self.depth = depth;
        self.world = build_floor(self.seed, depth);
        self.player.pos = self.world.marker("stairs_up").expect("floors have stairs");
        let cells = self.world.terrain.cells().len();
        self.explored = vec![false; cells];
        self.visible = vec![false; cells];
        self.monsters.clear();
        self.items.clear();
        self.populate();
    }

    /// Spawns from a per-floor seed, so placement never depends on play.
    fn populate(&mut self) {
        let depth = self.depth;
        let mut rng = Rng::new(self.seed.derive(&format!("spawn:{depth}")).0);
        let floor = self.world.tags.get("floor");
        let dist = self.world.distances_from(self.player.pos);
        let mut spots: Vec<Pos> = self
            .world
            .terrain
            .positions()
            .filter(|&p| {
                let i = self.world.terrain.index(p).unwrap();
                self.world.terrain.get(p) == floor && dist[i].is_some_and(|d| d >= 6)
            })
            .collect();
        let mut take =
            |rng: &mut Rng| (!spots.is_empty()).then(|| spots.swap_remove(rng.index(spots.len())));

        let eligible: Vec<usize> = (0..MONSTERS.len())
            .filter(|&k| MONSTERS[k].min_depth <= depth)
            .collect();
        let pool = &eligible[eligible.len().saturating_sub(4)..];
        for _ in 0..3 + depth * 3 / 4 {
            let Some(pos) = take(&mut rng) else { break };
            let kind = pool[rng.index(pool.len())];
            self.monsters.push(Monster {
                kind,
                pos,
                hp: MONSTERS[kind].hp,
                awake: false,
            });
        }

        let mut loot = vec![ItemKind::Potion];
        if rng.chance(0.5) {
            loot.push(ItemKind::Potion);
        }
        for _ in 0..3 + rng.index(3) {
            loot.push(ItemKind::Gold(rng.range(2, 6) as u32 + depth * 2));
        }
        if rng.chance(0.35) {
            loot.push(ItemKind::Sword);
        }
        if rng.chance(0.35) {
            loot.push(ItemKind::Shield);
        }
        for kind in loot {
            let Some(pos) = take(&mut rng) else { break };
            self.items.push(Item { kind, pos });
        }
    }

    fn step(&mut self, action: Action) -> bool {
        let (dx, dy) = action.delta().expect("movement action");
        let to = Pos {
            x: self.player.pos.x + dx,
            y: self.player.pos.y + dy,
        };
        if let Some(i) = self.monster_at(to) {
            self.attack(i);
            return true;
        }
        if !self.world.walkable(to) {
            return false;
        }
        self.player.pos = to;
        self.pick_up();
        true
    }

    fn descend(&mut self) -> bool {
        if self.world.marker("stairs_down") != Some(self.player.pos) {
            self.say("There are no stairs down here.");
            return false;
        }
        if self.depth == FLOORS {
            self.status = Status::Won;
            self.say("You reach the bottom of the undercroft. You win!");
            return true;
        }
        self.player.max_hp += 3;
        self.player.hp = (self.player.hp + self.player.max_hp / 4).min(self.player.max_hp);
        self.enter_floor(self.depth + 1);
        self.say(format!("You descend to floor {}.", self.depth));
        true
    }

    fn quaff(&mut self) -> bool {
        if self.potions == 0 {
            self.say("You have no potions.");
            return false;
        }
        if self.player.hp == self.player.max_hp {
            self.say("You're already at full health.");
            return false;
        }
        self.potions -= 1;
        let heal = 8 + self.depth as i32;
        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
        self.say("You drink a potion. You feel better.");
        true
    }

    fn pick_up(&mut self) {
        let Some(i) = self.items.iter().position(|it| it.pos == self.player.pos) else {
            return;
        };
        match self.items.remove(i).kind {
            ItemKind::Potion => {
                self.potions += 1;
                self.say("You pick up a potion.");
            }
            ItemKind::Gold(n) => {
                self.gold += n;
                self.say(format!("You find {n} gold."));
            }
            ItemKind::Sword => {
                self.player.atk += 1;
                self.say("You find a sharper sword. Attack +1.");
            }
            ItemKind::Shield => {
                self.player.def += 1;
                self.say("You find a sturdier shield. Defense +1.");
            }
        }
    }

    fn attack(&mut self, i: usize) {
        let kind: &'static MonsterKind = &MONSTERS[self.monsters[i].kind];
        let damage = (self.player.atk + self.rng.range(0, 3) - kind.def).max(1);
        let monster = &mut self.monsters[i];
        monster.hp -= damage;
        monster.awake = true;
        if monster.hp <= 0 {
            self.monsters.remove(i);
            self.kills += 1;
            self.say(format!("You slay the {}.", kind.name));
        } else {
            self.say(format!("You hit the {} for {damage}.", kind.name));
        }
    }

    fn monsters_act(&mut self) {
        let dist = self.world.distances_from(self.player.pos);
        for i in 0..self.monsters.len() {
            if self.status != Status::Playing {
                break;
            }
            let kind: &'static MonsterKind = &MONSTERS[self.monsters[i].kind];
            if kind.slow && self.turn % 2 == 0 {
                continue;
            }
            let pos = self.monsters[i].pos;
            let here = self.world.terrain.index(pos).unwrap();
            let near = pos.manhattan(self.player.pos);
            if !self.monsters[i].awake && self.visible[here] && near <= kind.sight {
                self.monsters[i].awake = true;
            }
            if !self.monsters[i].awake {
                if self.rng.chance(0.25) {
                    self.wander(i);
                }
                continue;
            }
            if near == 1 {
                self.monster_attack(kind);
                continue;
            }
            if kind.erratic && self.rng.chance(0.5) {
                self.wander(i);
                continue;
            }
            let closer = pos
                .neighbors4()
                .into_iter()
                .filter(|&n| self.free(n))
                .filter_map(|n| dist[self.world.terrain.index(n)?].map(|d| (d, n)))
                .min_by_key(|&(d, _)| d);
            if let (Some(d_here), Some((d, n))) = (dist[here], closer) {
                if d < d_here {
                    self.monsters[i].pos = n;
                }
            }
        }
    }

    fn monster_attack(&mut self, kind: &'static MonsterKind) {
        let damage = (kind.atk + self.rng.range(0, 2) - self.player.def).max(1);
        self.player.hp -= damage;
        if self.player.hp <= 0 {
            self.player.hp = 0;
            self.status = Status::Dead;
            self.killed_by = Some(kind.name);
            self.say(format!(
                "The {} kills you on floor {}.",
                kind.name, self.depth
            ));
        } else {
            self.say(format!("The {} hits you for {damage}.", kind.name));
        }
    }

    fn wander(&mut self, i: usize) {
        let to = self.monsters[i].pos.neighbors4()[self.rng.index(4)];
        if self.free(to) {
            self.monsters[i].pos = to;
        }
    }

    fn free(&self, p: Pos) -> bool {
        self.world.walkable(p) && p != self.player.pos && self.monster_at(p).is_none()
    }

    fn monster_at(&self, p: Pos) -> Option<usize> {
        self.monsters.iter().position(|m| m.pos == p)
    }

    fn update_fov(&mut self) {
        self.visible.fill(false);
        let (visible, explored) = (&mut self.visible, &mut self.explored);
        fov::compute(&self.world, self.player.pos, SIGHT, |i| {
            visible[i] = true;
            explored[i] = true;
        });
    }
}
