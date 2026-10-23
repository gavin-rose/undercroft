use undercroft::Seed;
use undercroft_sim::{Action, Game, Status, bot::Bot, decode, encode};

fn play(seed: Seed, loot: bool) -> Game {
    let mut game = Game::new(seed);
    let mut bot = Bot::new(loot);
    while game.status() == Status::Playing && game.view().turn < 20_000 {
        game.act(bot.next(&game));
    }
    game
}

#[test]
fn replays_reproduce_runs_exactly() {
    for i in 0..5 {
        let seed = Seed::from_text(&format!("replay-{i}"));
        let game = play(seed, true);
        let again = Game::replay(seed, &decode(&encode(game.actions())).unwrap());
        assert_eq!(again.status(), game.status());
        assert_eq!(again.depth(), game.depth());
        assert_eq!(again.score(), game.score());
        assert_eq!(again.view().player, game.view().player);
    }
}

#[test]
fn encoding_round_trips() {
    let actions = [Action::Right; 12]
        .into_iter()
        .chain([Action::Up, Action::Quaff, Action::Descend])
        .collect::<Vec<_>>();
    assert_eq!(encode(&actions), "r12uqx");
    assert_eq!(decode("r12uqx").unwrap(), actions);
    assert!(decode("r1z").is_none());
}

#[test]
fn blocked_moves_take_no_turn() {
    for d in [Action::Up, Action::Down, Action::Left, Action::Right] {
        let mut game = Game::new(Seed::from_text("walls"));
        for _ in 0..60 {
            if !game.act(d) {
                break;
            }
        }
        assert_eq!(game.view().turn as usize, game.actions().len());
    }
}

#[test]
fn floors_are_the_same_however_you_play() {
    let seed = Seed::from_text("fair");
    let a = play(seed, false);
    let b = play(seed, true);
    if a.depth() >= 2 && b.depth() >= 2 {
        let first = |g: &Game| Game::replay(seed, &g.actions()[..first_descent(g)]);
        assert_eq!(first(&a).world().terrain, first(&b).world().terrain);
    }
}

fn first_descent(game: &Game) -> usize {
    game.actions()
        .iter()
        .position(|&a| a == Action::Descend)
        .unwrap()
        + 1
}

#[test]
fn bots_survive_the_first_floor() {
    let runs = 20;
    let cleared = (0..runs)
        .filter(|i| play(Seed::from_text(&format!("sanity-{i}")), true).depth() > 1)
        .count();
    assert!(
        cleared >= runs * 4 / 5,
        "only {cleared}/{runs} cleared floor 1"
    );
}
