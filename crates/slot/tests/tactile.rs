mod common;
use slot::{app::App, audio::Sfx};
use slot_input::{Action, Btn};

#[test]
fn synthesized_clips_are_short_quiet_stereo_and_distinct() {
    let sounds = [
        Sfx::Cartridge(0),
        Sfx::Cartridge(1),
        Sfx::HeadphoneIn,
        Sfx::HeadphoneOut,
    ];
    let mut clips = Vec::new();
    for sound in sounds {
        for rate in [32_768, 48_000] {
            let clip = sound.render(rate);
            assert!(!clip.is_empty());
            assert!(clip.len() < rate as usize / 4);
            assert!(clip.chunks_exact(2).all(|p| p[0] == p[1]));
            let peak = clip.iter().map(|v| v.unsigned_abs()).max().unwrap();
            assert!((100..6000).contains(&peak), "{sound:?}: {peak}");
            assert!(clip[0].abs() < 10);
            assert!(clip.last().unwrap().abs() < 10);
        }
        clips.push(sound.render(48_000));
    }
    assert_ne!(clips[0], clips[1]);
    assert_ne!(clips[2], clips[3]);
    if let Ok(dir) = std::env::var("SLOT_TACTILE_PREVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, clip) in [
            "cartridge",
            "cartridge-variant",
            "headphone-in",
            "headphone-out",
        ]
        .into_iter()
        .zip(clips)
        {
            let bytes: Vec<_> = clip.into_iter().flat_map(i16::to_le_bytes).collect();
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.pcm")),
                bytes,
            )
            .unwrap();
        }
    }
}

#[test]
fn cartridge_motion_clicks_but_idle_and_disabled_sound_do_not() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta", "Gamma"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    a.update(0.1);
    assert!(a.take_sfx().is_none());
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    let mut heard = Vec::new();
    for _ in 0..40 {
        a.update(0.016);
        if let Some(s) = a.take_sfx() {
            heard.push(s);
        }
    }
    assert_eq!(heard.len(), 1);
    assert!(matches!(heard[0], Sfx::Cartridge(_)));
    let mut c = slot::comfort::Comfort::read(d.path(), &slot_store::SlotState::default()).unwrap();
    c.sounds = false;
    c.save(d.path()).unwrap();
    let mut a = App::boot(d.path());
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    for _ in 0..40 {
        a.update(0.016);
        assert!(a.take_sfx().is_none());
    }
}

#[test]
fn bluetooth_cues_are_once_per_transition_and_quiet_disconnect_stays_silent() {
    let d = common::tmp_root_with_carts(&["Alpha", "Beta"]);
    common::clocked(d.path());
    let mut a = App::boot(d.path());
    let address = Some("AA:BB:CC:DD:EE:FF".into());
    a.bluetooth_state(address.clone(), true);
    a.update(0.6);
    assert_eq!(a.take_sfx(), Some(Sfx::HeadphoneIn));
    a.bluetooth_state(address.clone(), true);
    a.update(0.6);
    assert!(a.take_sfx().is_none());
    a.bluetooth_state(address.clone(), false);
    a.update(0.6);
    assert_eq!(a.take_sfx(), Some(Sfx::HeadphoneOut));
    a.bluetooth_state(address.clone(), true);
    a.update(0.6);
    a.take_sfx();
    a.set_quiet_disconnect(true);
    a.bluetooth_state(address, false);
    a.bluetooth_state(None, false);
    a.update(0.6);
    assert_eq!(a.output_volume(), 0);
    assert!(a.take_sfx().is_none());
}
