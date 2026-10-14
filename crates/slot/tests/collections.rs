use slot::app::{App, Phase};
use slot::collections::{CartKey, Collections, RECENT_LIMIT};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, Cart, Platform, SlotState};
use slot_ui::{Shelf, TexId};
use std::fs;

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.into(),
        rom: format!(
            "Games/{}/{stem}.{}",
            platform.dir_name(),
            platform.extensions()[0]
        )
        .into(),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}
fn root() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    for (p, s) in [
        (Platform::Gba, "Same"),
        (Platform::Gb, "Same"),
        (Platform::Gbc, "Zelda"),
    ] {
        let c = cart(p, s);
        let path = d.path().join(c.rom);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0; 0x150]).unwrap();
    }
    fs::create_dir_all(d.path().join("Config")).unwrap();
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            show_recents: true,
            ..Default::default()
        },
    )
    .unwrap();
    d
}
fn tap(app: &mut App, btn: Btn) {
    app.apply(Action::GbaDown(btn));
    app.apply(Action::GbaUp(btn));
}
fn launch(app: &mut App) {
    app.apply(Action::Insert);
    app.on_core_ready();
    for _ in 0..100 {
        app.update(1.0 / 60.0);
    }
}
fn eject(app: &mut App) {
    app.apply(Action::Eject);
    for _ in 0..100 {
        app.update(1.0 / 60.0);
    }
}

#[test]
fn membership_roundtrips_and_keeps_missing_roms_without_touching_studio_files() {
    let d = root();
    fs::create_dir_all(d.path().join("Labels/GBA")).unwrap();
    let files = [
        ("Labels/GBA/Same.png", b"custom studio label".as_slice()),
        ("Labels/cart_shell.txt", b"Same = rounded ff3344 clear\n"),
        ("Config/cart_shell.txt", b"Same = notched 112233 solid\n"),
    ];
    for (path, data) in files {
        fs::write(d.path().join(path), data).unwrap();
    }
    let mut c = Collections::default();
    c.toggle(&cart(Platform::Gba, "Same"));
    c.toggle(&cart(Platform::Gb, "Same"));
    c.toggle(&cart(Platform::Gbc, "Missing"));
    c.save_favorites(d.path()).unwrap();
    c.launched(&cart(Platform::Gbc, "Missing"));
    c.save_recent(d.path()).unwrap();
    let loaded = Collections::read(d.path()).unwrap();
    assert_eq!(loaded.favorites, c.favorites);
    let carts = slot_store::scan(d.path()).unwrap();
    assert_eq!(loaded.shelves(&carts).1.len(), 2);
    assert_eq!(loaded.favorites.len(), 3);
    assert!(loaded.shelves(&carts).0.is_empty());
    let mut returned = carts.clone();
    returned.push(cart(Platform::Gbc, "Missing"));
    assert_eq!(loaded.shelves(&returned).0.len(), 1);
    assert_eq!(loaded.shelves(&returned).1.len(), 3);
    let original = carts.iter().find(|c| c.platform == Platform::Gba).unwrap();
    let virtual_cart = loaded
        .shelves(&carts)
        .1
        .into_iter()
        .find(|c| c.platform == Platform::Gba)
        .unwrap();
    assert_eq!(*original, virtual_cart);
    for (path, data) in files {
        assert_eq!(fs::read(d.path().join(path)).unwrap(), data);
    }
}
#[test]
fn recent_is_newest_first_unique_and_capped_and_favorites_are_alphabetic() {
    let mut c = Collections::default();
    for i in 0..40 {
        c.launched(&cart(Platform::Gba, &format!("Game {i}")));
    }
    c.launched(&cart(Platform::Gba, "Game 20"));
    assert_eq!(c.recent.len(), RECENT_LIMIT);
    assert_eq!(c.recent[0].stem, "Game 20");
    assert_eq!(c.recent[1].stem, "Game 39");
    assert_eq!(c.recent.iter().filter(|k| k.stem == "Game 20").count(), 1);
    let carts = vec![
        cart(Platform::Gb, "Zelda"),
        cart(Platform::Gba, "Alpha"),
        cart(Platform::Gbc, "Beta"),
    ];
    for cart in &carts {
        c.toggle(cart);
    }
    assert_eq!(
        c.shelves(&carts)
            .1
            .iter()
            .map(|c| c.stem.as_str())
            .collect::<Vec<_>>(),
        ["Alpha", "Beta", "Zelda"]
    );
}
#[test]
fn parser_ignores_bad_rows_and_duplicates_and_accepts_system_case() {
    let d = root();
    fs::write(
        d.path().join("Config/favourites"),
        "gba/Same\nGBA/Same\ngb/Same\ninvalid\ngba/../bad\ngba/..\ngba/\nxyz/game\n",
    )
    .unwrap();
    let c = Collections::read(d.path()).unwrap();
    assert_eq!(c.favorites.len(), 2);
}
#[test]
fn recent_boot_and_letter_jump_and_seated_cart_priority() {
    let d = root();
    fs::write(d.path().join("Config/recent"), "gb/Same\ngbc/Zelda\n").unwrap();
    let mut app = App::boot(d.path());
    assert_eq!(app.shelf_platform_name(), Some("Recents"));
    tap(&mut app, Btn::Down);
    assert_eq!(app.selected_stem(), Some("Same"));
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            show_recents: true,
            cart: Some("Zelda".into()),
            cart_platform: Some(Platform::Gbc),
            ..Default::default()
        },
    )
    .unwrap();
    let app = App::boot(d.path());
    assert_eq!(app.seated_cart().unwrap().platform, Platform::Gbc);
}
#[test]
fn y_toggles_last_favorite_skips_empty_shelf_and_persists() {
    let d = root();
    let mut app = App::boot(d.path());
    tap(&mut app, Btn::Select);
    assert!(!app.selected_is_favorite());
    tap(&mut app, Btn::Y);
    assert!(app.selected_is_favorite());
    for _ in 0..90 {
        app.update(1.0 / 60.0);
    }
    tap(&mut app, Btn::L1);
    assert_eq!(app.shelf_platform_name(), Some("Favorites"));
    tap(&mut app, Btn::Y);
    assert!(app.selected_is_favorite());
    tap(&mut app, Btn::A); // Keep is the safe default.
    assert!(app.selected_is_favorite());
    tap(&mut app, Btn::Y);
    tap(&mut app, Btn::Down);
    tap(&mut app, Btn::A);
    assert!(!app.selected_is_favorite());
    assert_eq!(app.shelf_platform_name(), Some("Favorites"));
    for _ in 0..120 {
        app.update(1.0 / 60.0);
    }
    assert_eq!(app.shelf_platform_name(), Some("Game Boy Advance"));
    assert!(Collections::read(d.path()).unwrap().favorites.is_empty());
}
#[test]
fn failed_load_is_not_recent_and_successful_load_from_mixed_shelf_is_correct() {
    let d = root();
    fs::write(d.path().join("Config/favourites"), "gba/Same\ngb/Same\n").unwrap();
    let mut app = App::boot(d.path());
    tap(&mut app, Btn::L1);
    assert_eq!(app.shelf_platform_name(), Some("Favorites"));
    tap(&mut app, Btn::Right);
    app.apply(Action::Insert);
    assert_eq!(app.seated_cart().unwrap().platform, Platform::Gb);
    assert!(Collections::read(d.path()).unwrap().recent.is_empty());
    app.on_core_failed();
    for _ in 0..100 {
        app.update(1.0 / 60.0);
    }
    assert!(Collections::read(d.path()).unwrap().recent.is_empty());
    launch(&mut app);
    assert_eq!(app.seated_cart().unwrap().platform, Platform::Gb);
    assert_eq!(
        Collections::read(d.path()).unwrap().recent[0],
        CartKey::of(&cart(Platform::Gb, "Same"))
    );
    eject(&mut app);
    assert!(matches!(app.phase(), Phase::Shelf));
    tap(&mut app, Btn::L1);
    assert_eq!(app.shelf_platform_name(), Some("Recents"));
    assert_eq!(
        app.browsed_rom().unwrap(),
        d.path().join("Games/GB/Same.gb")
    );
}
#[test]
fn launching_from_recent_reorders_without_switching_the_seated_cart() {
    let d = root();
    fs::write(
        d.path().join("Config/recent"),
        "gba/Same\ngb/Same\ngbc/Zelda\n",
    )
    .unwrap();
    let mut app = App::boot(d.path());
    tap(&mut app, Btn::Right);
    launch(&mut app);
    assert_eq!(app.seated_cart().unwrap().platform, Platform::Gb);
    assert_eq!(
        Collections::read(d.path()).unwrap().recent[0].platform,
        Platform::Gb
    );
    assert_eq!(app.carts().count(), 3);
}
#[test]
fn unreadable_membership_is_not_overwritten_or_reported_as_saved() {
    let d = root();
    fs::create_dir(d.path().join("Config/favourites")).unwrap();
    let mut app = App::boot(d.path());
    tap(&mut app, Btn::Y);
    assert!(!app.selected_is_favorite());
    assert!(d.path().join("Config/favourites").is_dir());
}
#[test]
fn texture_reordering_and_same_name_selection_stay_with_their_platform() {
    let a = cart(Platform::Gba, "Same");
    let b = cart(Platform::Gb, "Same");
    let mut shelf = Shelf::new(vec![a.clone(), b.clone()]);
    shelf.set_faces(vec![TexId::from_raw(10), TexId::from_raw(20)]);
    shelf.select(1);
    assert_eq!(shelf.find("Same").unwrap().0.platform, Platform::Gb);
    assert!(shelf.replace_carts(vec![b, a]).is_empty());
    assert_eq!(shelf.index, 0);
    assert_eq!(shelf.face(0), Some(TexId::from_raw(20)));
    assert_eq!(shelf.face(1), Some(TexId::from_raw(10)));
    let removed = shelf.replace_carts(vec![cart(Platform::Gba, "Same")]);
    assert_eq!(removed, vec![TexId::from_raw(20)]);
    assert_eq!(shelf.face(0), Some(TexId::from_raw(10)));
}
