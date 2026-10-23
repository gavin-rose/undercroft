//! A simple automatic player, used for balance testing.

use undercroft::{Pos, World};

use crate::game::{Action, Game};

const DIRS: [Action; 4] = [Action::Up, Action::Right, Action::Down, Action::Left];

/// Fights whatever is adjacent, drinks when low, and heads for the stairs.
/// With `loot`, it first detours for items it has seen nearby.
#[derive(Default)]
pub struct Bot {
    pub loot: bool,
    target: Option<(Pos, u32)>,
}

impl Bot {
    pub fn new(loot: bool) -> Self {
        Bot { loot, target: None }
    }

    pub fn next(&mut self, game: &Game) -> Action {
        let view = game.view();
        let p = view.player;
        if let Some(&d) = DIRS
            .iter()
            .find(|&&d| view.monsters.iter().any(|m| m.pos == step(p, d)))
        {
            return d;
        }
        if view.hp * 100 / view.max_hp < 45 && view.potions > 0 {
            return Action::Quaff;
        }
        let world = game.world();
        if self.loot {
            if self
                .target
                .is_some_and(|(pos, depth)| pos == p || depth != view.depth)
            {
                self.target = None;
            }
            if self.target.is_none() {
                let near = world.distances_from(p);
                self.target = view
                    .items
                    .iter()
                    .filter_map(|it| Some((near[world.terrain.index(it.pos)?]?, it.pos)))
                    .filter(|&(d, _)| d <= 12)
                    .min_by_key(|&(d, _)| d)
                    .map(|(_, pos)| (pos, view.depth));
            }
            if let Some(action) = self.target.and_then(|(pos, _)| toward(world, p, pos)) {
                return action;
            }
        }
        if view.on_stairs {
            return Action::Descend;
        }
        let stairs = world.marker("stairs_down").expect("floors have stairs");
        toward(world, p, stairs).unwrap_or(Action::Wait)
    }
}

fn step(p: Pos, d: Action) -> Pos {
    match d {
        Action::Up => Pos { x: p.x, y: p.y - 1 },
        Action::Right => Pos { x: p.x + 1, y: p.y },
        Action::Down => Pos { x: p.x, y: p.y + 1 },
        _ => Pos { x: p.x - 1, y: p.y },
    }
}

fn toward(world: &World, from: Pos, target: Pos) -> Option<Action> {
    let dist = world.distances_from(target);
    DIRS.into_iter()
        .filter_map(|d| Some((dist[world.terrain.index(step(from, d))?]?, d)))
        .min_by_key(|&(d, _)| d)
        .map(|(_, d)| d)
}
