use undercroft::{Error, Recipe, STYLES, Seed, generate};

fn style(name: &str) -> Recipe {
    Recipe::style(name).unwrap()
}

#[test]
fn every_style_generates_deterministically() {
    for &(name, _) in STYLES {
        let recipe = style(name);
        for i in 0..50 {
            let seed = Seed::from_text(&i.to_string());
            let a = generate(seed, &recipe).unwrap_or_else(|e| panic!("{name}, seed {i}: {e}"));
            let b = generate(seed, &recipe).unwrap();
            assert_eq!(a, b, "{name}, seed {i}");
        }
    }
}

#[test]
fn different_seeds_give_different_worlds() {
    let recipe = style("dungeon");
    let a = generate(Seed::from_text("a"), &recipe).unwrap();
    let b = generate(Seed::from_text("b"), &recipe).unwrap();
    assert_ne!(a.terrain, b.terrain);
}

#[test]
fn underground_styles_are_connected_with_distant_stairs() {
    for name in ["dungeon", "caves"] {
        for i in 0..50 {
            let world = generate(Seed::from_text(&i.to_string()), &style(name)).unwrap();
            assert_eq!(world.regions().len(), 1, "{name}, seed {i}");
            let up = world.marker("stairs_up").expect("up stairs");
            let down = world.marker("stairs_down").expect("down stairs");
            let dist = world.distances_from(up)[world.terrain.index(down).unwrap()];
            assert!(dist.is_some_and(|d| d >= 40), "{name}, seed {i}: {dist:?}");
        }
    }
}

#[test]
fn adding_a_stage_does_not_reshuffle_the_others() {
    let base = r#"
        gen_version = 1
        width = 32
        height = 16
        [[stage]]
        use = "noise_field"
        field = "height"
        scale = 8.0
    "#;
    let with_extra =
        format!("{base}\n[[stage]]\nuse = \"random_fill\"\ntag = \"floor\"\nchance = 0.5\n");
    let with_extra_first = r#"
        gen_version = 1
        width = 32
        height = 16
        [[stage]]
        use = "random_fill"
        tag = "floor"
        chance = 0.5
        [[stage]]
        use = "noise_field"
        field = "height"
        scale = 8.0
    "#;
    let seed = Seed::from_text("stable");
    let height = |src: &str| {
        let world = generate(seed, &Recipe::from_toml(src).unwrap()).unwrap();
        world.fields["height"].clone()
    };
    assert_eq!(height(base), height(&with_extra));
    assert_eq!(height(base), height(with_extra_first));
}

#[test]
fn recipe_mistakes_are_reported() {
    let recipe = |stage: &str| {
        Recipe::from_toml(&format!(
            "gen_version = 1\nwidth = 20\nheight = 20\n[[stage]]\n{stage}"
        ))
        .unwrap()
    };
    let seed = Seed(0);

    let err = generate(seed, &recipe("use = \"nope\"")).unwrap_err();
    assert!(
        matches!(err, Error::UnknownStage(ref s) if s == "nope"),
        "{err}"
    );

    let err = generate(seed, &recipe("use = \"fill\"\ntga = \"wall\"")).unwrap_err();
    assert!(matches!(err, Error::StageParams { .. }), "{err}");

    let err = generate(seed, &recipe("use = \"fill\"\ntag = \"lava\"")).unwrap_err();
    assert!(matches!(err, Error::InvalidRecipe(_)), "{err}");
}

#[test]
fn recipes_can_define_tags() {
    let src = r#"
        gen_version = 1
        width = 10
        height = 10
        [tags.lava]
        glyph = "%"
        [[stage]]
        use = "fill"
        tag = "lava"
    "#;
    let world = generate(Seed(0), &Recipe::from_toml(src).unwrap()).unwrap();
    assert!(world.to_string().starts_with("%%%%%%%%%%"));
}

/// If a snapshot changes, existing seeds now give different worlds: fix the
/// regression, or bump GEN_VERSION and accept the new snapshots.
#[test]
fn style_snapshots() {
    for &(name, _) in STYLES {
        let world = generate(Seed::from_text("undercroft"), &style(name)).unwrap();
        insta::assert_snapshot!(name, world.to_string());
    }
}
