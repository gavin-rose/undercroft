//! Plays many seeds with both bots and prints how far they get.
//! Run with `cargo run --release -p undercroft-sim --example balance [runs]`.

use undercroft::Seed;
use undercroft_sim::{FLOORS, Game, Status, bot::Bot};

fn main() {
    let runs: u32 = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(500);
    for loot in [false, true] {
        let mut deaths = vec![0u32; FLOORS as usize + 1];
        let (mut wins, mut turns) = (0, 0u64);
        for i in 0..runs {
            let mut game = Game::new(Seed::from_text(&format!("balance-{i}")));
            let mut bot = Bot::new(loot);
            while game.status() == Status::Playing && game.view().turn < 20_000 {
                game.act(bot.next(&game));
            }
            turns += game.view().turn as u64;
            match game.status() {
                Status::Won => wins += 1,
                _ => deaths[game.depth() as usize] += 1,
            }
        }
        println!(
            "{:<8} wins {:>3}%  avg turns {:>5}  deaths by floor {:?}",
            if loot { "looter" } else { "rusher" },
            wins * 100 / runs,
            turns / runs as u64,
            &deaths[1..]
        );
    }
}
