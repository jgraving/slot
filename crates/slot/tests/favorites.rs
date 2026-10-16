mod common;

use std::path::Path;

use common::{tmp_root_with_carts, write_gb_cart};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, Platform, Recent, SlotState};
use tempfile::TempDir;

fn card() -> TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn favorite(platform: Platform, stem: &str) -> Recent {
    Recent {
        platform,
        stem: stem.into(),
    }
}

fn boot_with(root: &Path, state: SlotState) -> App {
    write_slot_state(
        root,
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    App::boot(root)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go_to(a: &mut App, shelf: &str) {
    for _ in 0..8 {
        if a.shelf_platform_name() == Some(shelf) {
            return;
        }
        press(a, Btn::R1);
    }
    panic!("never reached the {shelf} shelf");
}

fn names(a: &mut App) -> Vec<&'static str> {
    let mut names = Vec::new();
    for _ in 0..4 {
        press(a, Btn::R1);
        names.extend(a.shelf_platform_name());
    }
    names
}

#[test]
fn y_on_the_shelf_stars_the_cart_and_saves_it_without_a_banner() {
    let d = card();
    let mut a = boot_with(d.path(), SlotState::default());
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    let stem = a.selected_stem().unwrap().to_string();
    press(&mut a, Btn::Y);
    assert_eq!(
        read_slot_state(d.path()).favorites,
        [favorite(Platform::Gba, &stem)]
    );
    assert_eq!(a.toast(), None);
    assert_eq!(
        a.selected_stem(),
        Some(stem.as_str()),
        "starring moved the shelf"
    );
}

#[test]
fn the_favorites_shelf_only_shows_once_something_is_starred() {
    let d = card();
    let mut a = boot_with(d.path(), SlotState::default());
    assert!(!names(&mut a).contains(&"Favorites"));
    go_to(&mut a, "Game Boy");
    press(&mut a, Btn::Y);
    go_to(&mut a, "Favorites");
    assert_eq!(a.selected_stem(), Some("Tetris"));
}

#[test]
fn favorites_from_every_system_share_one_shelf_in_name_order() {
    let d = card();
    let mut a = boot_with(
        d.path(),
        SlotState {
            favorites: vec![
                favorite(Platform::Gb, "Tetris"),
                favorite(Platform::Gba, "Fusion"),
                favorite(Platform::Gba, "Emerald"),
            ],
            ..SlotState::default()
        },
    );
    go_to(&mut a, "Favorites");
    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(a.selected_stem().unwrap().to_string());
        a.apply(Action::ShelfRight);
        a.apply(Action::GbaUp(Btn::Right));
        for _ in 0..60 {
            a.update(1.0 / 60.0);
        }
    }
    assert_eq!(seen, ["Emerald", "Fusion", "Tetris"]);
}

#[test]
fn unstarring_on_the_favorites_shelf_takes_the_cart_off_it() {
    let d = card();
    let mut a = boot_with(
        d.path(),
        SlotState {
            favorites: vec![
                favorite(Platform::Gba, "Emerald"),
                favorite(Platform::Gb, "Tetris"),
            ],
            ..SlotState::default()
        },
    );
    go_to(&mut a, "Favorites");
    assert_eq!(a.selected_stem(), Some("Emerald"));
    press(&mut a, Btn::Y);
    assert!(a.favorite_removal_pending());
    assert_eq!(a.selected_stem(), Some("Emerald"));
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
    press(&mut a, Btn::Y);
    assert_eq!(a.toast(), None);
    assert_eq!(a.shelf_platform_name(), Some("Favorites"));
    assert_eq!(a.selected_stem(), Some("Tetris"));
    assert_eq!(
        read_slot_state(d.path()).favorites,
        [favorite(Platform::Gb, "Tetris")]
    );
}

#[test]
fn unstarring_the_last_favorite_leaves_the_empty_shelf() {
    let d = card();
    let mut a = boot_with(
        d.path(),
        SlotState {
            favorites: vec![favorite(Platform::Gb, "Tetris")],
            ..SlotState::default()
        },
    );
    go_to(&mut a, "Favorites");
    press(&mut a, Btn::Y);
    assert_eq!(a.shelf_platform_name(), Some("Favorites"));
    assert_eq!(read_slot_state(d.path()).favorites.len(), 1);
    press(&mut a, Btn::Y);
    assert_ne!(a.shelf_platform_name(), Some("Favorites"));
    assert!(read_slot_state(d.path()).favorites.is_empty());
    assert!(!names(&mut a).contains(&"Favorites"));
}

#[test]
fn a_favorite_whose_rom_is_gone_is_left_off_the_shelf() {
    let d = card();
    let mut a = boot_with(
        d.path(),
        SlotState {
            favorites: vec![
                favorite(Platform::Gba, "Deleted"),
                favorite(Platform::Gba, "Fusion"),
            ],
            ..SlotState::default()
        },
    );
    go_to(&mut a, "Favorites");
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

fn two_favorites() -> (TempDir, App) {
    let d = card();
    let mut a = boot_with(d.path(), SlotState {
        favorites: vec![favorite(Platform::Gba, "Emerald"), favorite(Platform::Gba, "Fusion")],
        ..SlotState::default()
    });
    go_to(&mut a, "Favorites");
    (d, a)
}

#[test]
fn held_y_cannot_confirm_removal_without_a_release() {
    let (d, mut a) = two_favorites();
    for _ in 0..10 { a.apply(Action::GbaDown(Btn::Y)); }
    assert!(a.favorite_removal_pending());
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
    a.apply(Action::GbaUp(Btn::Y));
    a.apply(Action::GbaDown(Btn::Y));
    assert_eq!(read_slot_state(d.path()).favorites, [favorite(Platform::Gba, "Fusion")]);
}

#[test]
fn timeout_keeps_the_favorite_and_next_y_starts_over() {
    let (d, mut a) = two_favorites();
    press(&mut a, Btn::Y);
    a.update(3.1);
    assert!(!a.favorite_removal_pending());
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
    press(&mut a, Btn::Y);
    assert!(a.favorite_removal_pending());
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
}

#[test]
fn navigating_or_leaving_the_shelf_cancels_without_saving_removal() {
    for action in [Action::ShelfRight, Action::ShelfLeft, Action::GbaDown(Btn::R1),
        Action::GbaDown(Btn::L1), Action::GbaDown(Btn::B), Action::QuickMenu,
        Action::GbaDown(Btn::Start), Action::LidClose, Action::Insert] {
        let (d, mut a) = two_favorites();
        press(&mut a, Btn::Y);
        a.apply(action);
        assert!(!a.favorite_removal_pending(), "{action:?}");
        assert_eq!(read_slot_state(d.path()).favorites.len(), 2, "{action:?}");
    }
}

#[test]
fn selecting_another_cart_cannot_confirm_the_previous_one() {
    let (d, mut a) = two_favorites();
    press(&mut a, Btn::Y);
    a.apply(Action::ShelfRight);
    a.apply(Action::GbaUp(Btn::Right));
    for _ in 0..60 { a.update(1.0 / 60.0); }
    assert_eq!(a.selected_stem(), Some("Fusion"));
    press(&mut a, Btn::Y);
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
    press(&mut a, Btn::Y);
    assert_eq!(read_slot_state(d.path()).favorites, [favorite(Platform::Gba, "Emerald")]);
}

#[test]
fn removing_from_a_system_shelf_also_requires_confirmation() {
    let (d, mut a) = two_favorites();
    go_to(&mut a, "Game Boy Advance");
    press(&mut a, Btn::Y);
    assert!(a.favorite_removal_pending());
    assert_eq!(read_slot_state(d.path()).favorites.len(), 2);
    press(&mut a, Btn::Y);
    assert_eq!(read_slot_state(d.path()).favorites.len(), 1);
}

#[test]
fn prompt_fades_above_the_selected_cart_then_disappears_on_cancel() {
    let (_d, mut a) = two_favorites();
    let tex = slot_ui::TexId::from_raw(777);
    let face = slot_ui::favorite_remove_face();
    a.set_favorite_remove_face(tex, face.w, face.h);
    press(&mut a, Btn::Y);
    a.update(0.08);
    let mut draws = vec![];
    a.draw(&mut draws);
    let first = draws.iter().find_map(|d| match d {
        slot_ui::Draw::Tex { tex: t, y, h, alpha, .. } if *t == tex => Some((*y, *h, *alpha)),
        _ => None,
    }).unwrap();
    assert!(first.0 + first.1 < slot_ui::rest_y(212.0));
    assert!(first.2 > 0.0 && first.2 < 1.0);
    a.update(0.12);
    draws.clear(); a.draw(&mut draws);
    assert!(draws.iter().any(|d| matches!(d, slot_ui::Draw::Tex { tex: t, alpha, .. } if *t == tex && *alpha == 1.0)));
    press(&mut a, Btn::B);
    draws.clear(); a.draw(&mut draws);
    assert!(!draws.iter().any(|d| matches!(d, slot_ui::Draw::Tex { tex: t, .. } if *t == tex)));
}
