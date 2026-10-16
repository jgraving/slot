use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slot_gfx::{Canvas, Layout, OUT_H, OUT_W};
use slot_input::{Action, Btn, MENU_DOUBLE_TAP_MS, MUTE_CHORD_MS};
use slot_power::{Battery, Charge, LedState, LidPolicy, Power};
use slot_retro::LinkChannel;
use slot_store::{
    format_stamp, parse_stamp, read_slot_state, scan, write_slot_state, BezelPick, Cart, Core,
    GbPalette, Platform, Recent, Shader, SlotState, StateEntry, StateRing, WallpaperPick,
    WallpaperTint, BLUE_LIGHT_MAX, BRIGHTNESS_MAX, FF_SPEEDS, RING_MAX, RUN_AHEAD_MAX, VOLUME_MAX,
};
use slot_ui::{
    board_from, board_zoom, draw_empty_slot, draw_footer, draw_ground, draw_slot_name,
    draw_sticker, ease, footer_ink, grown, lid_at, lid_from, lift_of, on_board, repeat_gap,
    shelf_cart_at, ClockPicker, Draw, FfState, GbShell, Hud, HudKind, Icon, LinkBadge, Millis,
    Placed, Polaroids, PowerChoice, QuickMenu, QuickMenuFaces, QuickRow, QuickValue, Refusal,
    Shelf, SlotChrome, TexId, Toast, CART_W, CHIP_H, CHIP_U, CHIP_V, CHIP_W, HINT_EDGE, HOP_LIFT,
    MOUTH_H, REPEAT_DELAY_MS, SHADOW_H, SHADOW_W, SOCKET_H, SOCKET_U, SOCKET_V, SOCKET_W, TURN_PAD,
};

use crate::audio::Sfx;
use crate::core_picker::{Chip, CorePicker, Outcome, Press};
use crate::link_kind::{link_carried, link_kind, serial_option, LinkKind};
use crate::link_radio::{radio_jobs, LinkRole, RadioJob, RadioJobs};
use crate::link_screen::LinkSprites;
use crate::link_start::{link_port, LinkFail, LinkProgress, LinkStarter, LinkStep};
use crate::persist::{self, Snapshot};
use crate::video_mode::{self, VideoMode};

pub const INSERT_S: f32 = 0.73;
const INSERT_HOLD_S: f32 = 0.28;

pub const SEATED_AT: f32 = INSERT_S - INSERT_HOLD_S;

pub const EJECT_S: f32 = SEATED_AT;

const EJECT_HOLD_S: f32 = 0.35;

pub const IN_GAME_MENU: Layout = Layout::Native;

const POWER_ON_S: f32 = 0.22;

const POWER_OFF_S: f32 = 0.16;

const FOOTER_SLIDE_S: f32 = POWER_ON_S;

const VOLUME_STEP: u8 = 5;

const AUTOSAVE_MS: Millis = 60_000;
const CARD_CHECK_MS: Millis = 2_000;

const BATTERY_CRITICAL: u8 = 5;

const BATTERY_POLL_MS: Millis = 10_000;
const MENU_REVEAL_MS: Millis = 180;

const CHARGE_POLL_MS: Millis = 1_000;
const HEADPHONES_POLL_MS: Millis = 500;

const BATTERY_LOW: u8 = 20;

pub const UNDO_GRACE_MS: Millis = 30_000;

pub const LINK_LOST_MS: Millis = 2000;

const PLAY_HOLD_MS: Millis = 500;

const SHUTDOWN_SHOW_MS: Millis = 250;

const CORE_PICKER_RECEDE: f32 = 0.26;

const SLOT_NAME_IN_MS: Millis = 200;
const SLOT_NAME_HOLD_MS: Millis = 1200;
const SLOT_NAME_OUT_MS: Millis = 800;
const SLOT_NAME_ALPHA: f32 = 1.0;
const CORE_PICKER_DIM: f32 = 0.25 / (1.0 - CORE_PICKER_RECEDE);
const CORE_LEGEND_Y: f32 = OUT_H as f32 - MOUTH_H * 2.0;
const CORE_LEGEND_GAP: f32 = 24.0;
const LINK_TEXT_Y: f32 = 44.0;
const LINK_LEGEND_Y: f32 = OUT_H as f32 - MOUTH_H;
pub const LINK_SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const LINK_LEGEND_GAP: f32 = 40.0;
const LINK_LEGEND_EDGE: f32 = 0.0;
const LID_SHADOW_W: f32 = 168.0;
const LID_SHADOW_H: f32 = 18.0;
const LID_SHADOW_DROP: f32 = 27.1;
const LID_SHADOW_ALPHA: f32 = 0.8;
const FACES_WAIT_MS: Millis = 1500;

const ALERT_HOLD: f32 = 0.45;
const ALERT_GONE: f32 = 0.9;

const CLOCK_FLOOR: i64 = 1_577_836_800;

pub enum PendingUndo {
    Save {
        stamp: String,
        evicted: Option<(String, Vec<u8>, Vec<u8>)>,
    },
    Load {
        prior: Vec<u8>,
    },
}

struct LinkSession {
    client_id: u16,
    lost_at: Option<Millis>,
}

struct LinkStarting {
    starter: LinkStarter,
    client_id: u16,
}

struct Reload {
    stem: String,
    role: LinkRow,
    cancelled: bool,
    from: LinkKind,
    from_serial: &'static str,
    fallback: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GameMenu {
    Pick(LinkRow),
    Working {
        role: LinkRow,
        step: LinkStep,
        since: Millis,
    },
    Linked {
        role: LinkRow,
        worked: Millis,
        since: Millis,
        opened: bool,
    },
    Failed {
        role: LinkRow,
        fail: LinkFail,
        worked: Millis,
        since: Millis,
    },
    Unplug {
        role: LinkRow,
        since: Millis,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkRow {
    Host,
    Join,
}

impl LinkRow {
    pub const ALL: [LinkRow; 2] = [LinkRow::Host, LinkRow::Join];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            LinkRow::Host => "Host",
            LinkRow::Join => "Join",
        }
    }

    pub fn role(self) -> LinkRole {
        match self {
            LinkRow::Host => LinkRole::Host,
            LinkRow::Join => LinkRole::Join,
        }
    }

    pub fn client_id(self) -> u16 {
        match self {
            LinkRow::Host => 0,
            LinkRow::Join => 1,
        }
    }

    pub fn other(self) -> LinkRow {
        match self {
            LinkRow::Host => LinkRow::Join,
            LinkRow::Join => LinkRow::Host,
        }
    }

    pub fn from_client_id(id: u16) -> LinkRow {
        if id == 0 {
            LinkRow::Host
        } else {
            LinkRow::Join
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkLegend {
    Cancel,
    Mode,
    Swap,
    Link,
    Ok,
    Back,
    EndLink,
}

impl LinkLegend {
    pub const ALL: [LinkLegend; 7] = [
        LinkLegend::Cancel,
        LinkLegend::Mode,
        LinkLegend::Swap,
        LinkLegend::Link,
        LinkLegend::Ok,
        LinkLegend::Back,
        LinkLegend::EndLink,
    ];

    pub fn index(self) -> usize {
        self as usize
    }
}

pub const LINKED_HOLD_MS: Millis = 1000;

pub const UNPLUG_HOLD_MS: Millis = 420;

#[derive(Debug)]
pub enum Phase {
    SetClock {
        picker: ClockPicker,
        seed: i64,
        from_menu: bool,
    },
    Shelf,
    QuickMenu {
        row: QuickRow,
    },
    Inserting {
        cart: String,
        t: f32,
        core_ready: bool,
        resumed: bool,
        clean: bool,
    },
    Playing {
        cart: String,
    },
    Ejecting {
        cart: String,
        t: f32,
    },
    Polaroids {
        cart: String,
    },
    About,
    Doze {
        cart: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Rack {
    Recents,
    Favorites,
    Of(Platform),
}

impl Rack {
    fn platform(self) -> Option<Platform> {
        match self {
            Rack::Of(p) => Some(p),
            Rack::Recents | Rack::Favorites => None,
        }
    }

    fn key(self) -> String {
        match self {
            Rack::Recents => "recents".into(),
            Rack::Favorites => "favorites".into(),
            Rack::Of(p) => p.dir_name().to_ascii_lowercase(),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Rack::Recents => "Recents",
            Rack::Favorites => "Favorites",
            Rack::Of(p) => p.name(),
        }
    }
}

const RECENTS_AT: usize = 0;
const FAVORITES_AT: usize = 1;

pub struct App {
    pub bluetooth: Option<crate::bluetooth::Manager>,
    bluetooth_supported: bool,
    pub audio_indicator: Option<TexId>,
    pub hud_audio_indicator: Option<TexId>,
    bluetooth_connected: bool,
    bluetooth_route: Option<String>,
    usb_active: bool,
    disconnect_volume: u8,
    bluetooth_probe: Option<std::sync::mpsc::Receiver<bool>>,
    phase: Phase,
    shelves: Vec<(Rack, Shelf)>,
    shelf_at: usize,
    orphaned: Vec<TexId>,
    play_held: Option<Millis>,
    held_dir: Option<(Btn, Millis, usize)>,
    refusal: Option<Refusal>,
    refused_from: Option<f32>,
    alert_face: Option<TexId>,
    shutdown_faces: Vec<(TexId, u32, u32)>,
    core_picker: Option<CorePicker>,
    core_board_face: Option<TexId>,
    core_interior_face: Option<TexId>,
    core_lid_face: Option<TexId>,
    core_faces_stem: Option<String>,
    core_socket_faces: Vec<TexId>,
    core_chip_faces: Vec<TexId>,
    core_blank_chip_face: Option<TexId>,
    core_chip_shadow_face: Option<TexId>,
    core_legend_faces: Vec<(TexId, u32)>,
    game_menu: Option<GameMenu>,
    link_sprites: Option<LinkSprites>,
    link_shot: Option<TexId>,
    link_hardware: LinkKind,
    last_role: LinkRow,
    link_choices: HashMap<String, LinkKind>,
    link_loaded: Option<&'static str>,
    link_reload: Option<(String, &'static str)>,
    reload: Option<Reload>,
    link_menu_faces: Vec<(TexId, u32, u32)>,
    link_linked_face: Option<(TexId, u32, u32)>,
    link_step_faces: Vec<(TexId, u32, u32)>,
    link_fail_faces: Vec<(TexId, u32, u32)>,
    link_legend_faces: Vec<(TexId, u32)>,
    starting: Option<LinkStarting>,
    link_transport: Option<(u16, Box<dyn LinkChannel>)>,
    restarting: bool,
    act_at: Millis,
    root: Option<PathBuf>,
    state: SlotState,
    vol_before: Vec<(u8, bool, Millis)>,
    snapshot: Option<Box<dyn Snapshot>>,
    core: Core,
    colour_pending: Option<bool>,
    palette_live: bool,
    palette_pending: Option<GbPalette>,
    link_player: Option<u8>,
    platform: Platform,
    named_core: bool,
    video_mode: VideoMode,
    link: Option<LinkSession>,
    sfx: Option<Sfx>,
    polaroids: Option<Polaroids>,
    pending: Option<(PendingUndo, Millis)>,
    legend_faces: Vec<TexId>,
    undo_face: Option<TexId>,
    clock_faces: Option<(TexId, TexId)>,
    clock_back: Option<(TexId, u32)>,
    quick_menu_faces: Option<QuickMenuFaces>,
    quick_clock_faces: Option<[(TexId, u32, u32); 2]>,
    quick_wallpaper_faces: Option<[(TexId, u32, u32); 2]>,
    quick_palette_faces: Option<[(TexId, u32, u32); 2]>,
    quick_bezel_faces: [Option<[(TexId, u32, u32); 2]>; 3],
    game_bezel: Option<TexId>,
    in_game_menu: Option<QuickRow>,
    /// 0 while the pause footer is off the bottom, 1 when it sits in the carousel's place.
    footer: f32,
    footer_row: Option<QuickRow>,
    states_from_menu: bool,
    menu_tap: Option<(Millis, Option<QuickRow>)>,
    sticker_face: Option<TexId>,
    ground: Option<TexId>,
    blur: Option<TexId>,
    wallpaper: Option<TexId>,
    random_wallpaper: Option<Option<String>>,
    battery_percent: slot_ui::Printed,
    bolt: Option<TexId>,
    hud_bolt: Option<TexId>,
    slot_faces: Option<(TexId, TexId)>,
    shelf_platform: slot_ui::Printed,
    shelf_title: slot_ui::Printed,
    name_pending: bool,
    slot_letter: Option<char>,
    shelf_named: Option<Millis>,
    shelf_clock: slot_ui::Printed,
    hud: Hud,
    screen: f32,
    lift: f32,
    eject_overlay: f32,
    panel: (u32, u32),
    game_ready: bool,
    clock: f64,
    power: Option<Power>,
    dozed_at: Millis,
    woke_on_press: bool,
    autosave_at: Millis,
    pending_save: Option<std::thread::JoinHandle<()>>,
    card_writes: Arc<crate::card_log::CardWrites>,
    card_read_only: bool,
    card_check_at: Millis,
    battery_at: Millis,
    charge_at: Millis,
    headphones: bool,
    headphones_at: Millis,
    battery: Option<Battery>,
    last_led: Option<LedState>,
    powering_off: bool,
    radio: Box<dyn RadioJobs>,
    state_writer: crate::state_writer::StateWriter,
}

const FACE_AHEAD: i32 = 8;
const FACE_KEEP: i32 = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceWant {
    pub shelf: usize,
    pub index: usize,
    pub on_screen: bool,
}

fn shelves_of(carts: Vec<Cart>) -> Vec<(Rack, Shelf)> {
    let mut rows: Vec<(Rack, Vec<Cart>)> = [Rack::Recents, Rack::Favorites]
        .into_iter()
        .chain(Platform::ALL.iter().map(|p| Rack::Of(*p)))
        .map(|r| (r, Vec::new()))
        .collect();
    for cart in carts {
        if let Some((_, row)) = rows.iter_mut().find(|(r, _)| *r == Rack::Of(cart.platform)) {
            row.push(cart);
        }
    }
    rows.into_iter()
        .map(|(rack, carts)| (rack, Shelf::new(carts)))
        .collect()
}

fn shift_down(draws: &mut [Draw], dy: f32) {
    if dy == 0.0 {
        return;
    }
    for draw in draws {
        match draw {
            Draw::Rect { y, .. }
            | Draw::Tex { y, .. }
            | Draw::Part { y, .. }
            | Draw::Turned { y, .. } => *y += dy,
            Draw::Game | Draw::Shot { .. } => {}
        }
    }
}

impl App {
    pub fn new(carts: Vec<Cart>) -> Self {
        let shelves = shelves_of(carts);
        let shelf_at = shelves
            .iter()
            .position(|(_, s)| !s.carts.is_empty())
            .unwrap_or(0);
        let state_writer = crate::state_writer::StateWriter::spawn();
        let card_writes = state_writer.card_writes();
        App {
            bluetooth: None,
            bluetooth_supported: false,
            audio_indicator: None,
            hud_audio_indicator: None,
            bluetooth_connected: false,
            bluetooth_route: None,
            usb_active: false,
            disconnect_volume: 0,
            bluetooth_probe: None,
            radio: radio_jobs(),
            state_writer,
            phase: Phase::Shelf,
            shelves,
            shelf_at,
            orphaned: Vec::new(),
            play_held: None,
            held_dir: None,
            refusal: None,
            refused_from: None,
            alert_face: None,
            shutdown_faces: Vec::new(),
            core_picker: None,
            core_board_face: None,
            core_interior_face: None,
            core_lid_face: None,
            core_faces_stem: None,
            core_socket_faces: Vec::new(),
            core_chip_faces: Vec::new(),
            core_blank_chip_face: None,
            core_chip_shadow_face: None,
            core_legend_faces: Vec::new(),
            game_menu: None,
            link_sprites: None,
            link_shot: None,
            link_hardware: LinkKind::Cable,
            last_role: LinkRow::Host,
            link_choices: HashMap::new(),
            link_loaded: None,
            link_reload: None,
            reload: None,
            link_menu_faces: Vec::new(),
            link_linked_face: None,
            link_step_faces: Vec::new(),
            link_fail_faces: Vec::new(),
            link_legend_faces: Vec::new(),
            starting: None,
            link_transport: None,
            restarting: false,
            act_at: 0,
            root: None,
            state: SlotState::default(),
            vol_before: Vec::new(),
            snapshot: None,
            core: Core::default(),
            colour_pending: None,
            palette_live: false,
            palette_pending: None,
            link_player: None,
            platform: Platform::default(),
            named_core: false,
            video_mode: VideoMode::default(),
            link: None,
            sfx: None,
            polaroids: None,
            pending: None,
            legend_faces: Vec::new(),
            undo_face: None,
            clock_faces: None,
            clock_back: None,
            quick_menu_faces: None,
            quick_clock_faces: None,
            quick_wallpaper_faces: None,
            quick_palette_faces: None,
            quick_bezel_faces: [None, None, None],
            game_bezel: None,
            in_game_menu: None,
            footer: 0.0,
            footer_row: None,
            states_from_menu: false,
            menu_tap: None,
            sticker_face: None,
            ground: None,
            blur: None,
            wallpaper: None,
            random_wallpaper: None,
            battery_percent: slot_ui::Printed::default(),
            bolt: None,
            hud_bolt: None,
            slot_faces: None,
            shelf_platform: slot_ui::Printed::default(),
            shelf_title: slot_ui::Printed::default(),
            name_pending: false,
            slot_letter: None,
            shelf_named: None,
            shelf_clock: slot_ui::Printed::default(),
            hud: Hud::new(),
            screen: 0.0,
            lift: 1.0,
            eject_overlay: 0.0,
            panel: (0, 0),
            game_ready: false,
            clock: 0.0,
            power: None,
            dozed_at: 0,
            woke_on_press: false,
            autosave_at: AUTOSAVE_MS,
            pending_save: None,
            card_writes,
            card_read_only: false,
            card_check_at: 0,
            battery_at: BATTERY_POLL_MS,
            charge_at: CHARGE_POLL_MS,
            headphones: false,
            headphones_at: 0,
            battery: None,
            last_led: None,
            powering_off: false,
        }
    }

    pub fn boot(root: &Path) -> Self {
        crate::root::ensure(root);
        crate::boot_time::mark("root");
        let carts = scan(root).unwrap_or_default();
        crate::boot_time::mark("scan");
        let mut app = App::new(carts);
        app.root = Some(root.to_path_buf());
        app.bluetooth_probe = crate::bluetooth::probe(root);
        app.disconnect_volume = crate::bluetooth::disconnect_volume(root);
        app.state = read_slot_state(root);
        app.restock_recents();
        app.restock_favorites();
        if let Some((at, i)) = app.remembered_seat() {
            app.shelf_at = at;
            app.shelves[at].1.select(i);
        } else if !app.shelves[RECENTS_AT].1.carts.is_empty() {
            app.shelf_at = RECENTS_AT;
        }
        if app.state.clock_set {
            app.start();
        } else {
            app.phase = clock_screen(system_secs(), 0, false);
        }
        app
    }

    fn start(&mut self) {
        let seated = if self.single_cart() {
            Some((self.shelf_at, 0))
        } else {
            let stem = self.state.cart.clone();
            let platform = self.state.cart_platform;
            stem.and_then(|stem| {
                let shelf = &self.shelf().carts;
                let here = shelf
                    .iter()
                    .position(|c| c.stem == stem && platform.is_none_or(|p| c.platform == p));
                here.map(|i| (self.shelf_at, i))
                    .or_else(|| self.seat_of(&stem, platform))
            })
        };
        self.phase = Phase::Shelf;
        match seated {
            Some((at, i)) => {
                self.shelf_at = at;
                self.shelf_mut().select(i);
                self.insert(false);
                if let Phase::Inserting { resumed, t, .. } = &mut self.phase {
                    *resumed = true;
                    *t = INSERT_S;
                }
            }
            None => {
                self.state.cart = None;
                self.state.cart_platform = None;
                self.name_pending = true;
            }
        }
    }

    fn shelf(&self) -> &Shelf {
        &self.shelves[self.shelf_at].1
    }

    fn shelf_mut(&mut self) -> &mut Shelf {
        &mut self.shelves[self.shelf_at].1
    }

    fn switch_shelf(&mut self, by: i32) {
        let Some(to) = self.next_shelf(by) else {
            return;
        };
        self.shelf_mut().release_hold();
        self.play_held = None;
        self.shelf_at = to;
        self.shelf_platform = slot_ui::Printed::default();
        self.slot_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
    }

    fn track_held_dir(&mut self, action: Action) {
        match action {
            Action::GbaDown(b @ (Btn::Up | Btn::Down | Btn::Left | Btn::Right)) => {
                if self.held_dir.map(|(held, ..)| held) != Some(b) {
                    self.held_dir = Some((b, self.now() + REPEAT_DELAY_MS, 0));
                }
            }
            Action::GbaUp(b) if self.held_dir.is_some_and(|(held, ..)| held == b) => {
                self.held_dir = None;
            }
            _ => {}
        }
    }

    fn repeat_held_dir(&mut self) {
        let Some((btn, due, fired)) = self.held_dir else {
            return;
        };
        if self.now() < due {
            return;
        }
        if !self.repeats(btn) {
            self.held_dir = None;
            return;
        }
        self.held_dir = Some((btn, self.now() + repeat_gap(fired), fired + 1));
        self.apply(Action::GbaDown(btn));
    }

    fn repeats(&self, btn: Btn) -> bool {
        let vertical = matches!(btn, Btn::Up | Btn::Down);
        if self.game_menu.is_some() {
            return false;
        }
        match self.phase {
            Phase::SetClock { .. } => true,
            Phase::QuickMenu { .. } => vertical,
            Phase::Shelf => vertical && self.core_picker.is_none(),
            Phase::Polaroids { .. } => !vertical,
            Phase::Playing { .. } => vertical && self.in_game_menu.is_some(),
            _ => false,
        }
    }

    fn jump_letter(&mut self, dir: i32) {
        let before = self.shelf().index;
        match dir > 0 {
            true => self.shelf_mut().jump_next_letter(),
            false => self.shelf_mut().jump_prev_letter(),
        }
        if self.shelf().index == before {
            return;
        }
        let Some(letter) = self.selected_stem().map(slot_store::initial) else {
            return;
        };
        if self.slot_letter != Some(letter) {
            self.shelf_platform = slot_ui::Printed::default();
        }
        self.slot_letter = Some(letter);
        self.shelf_named = None;
        self.name_pending = true;
    }

    fn slot_name_alpha(&self) -> f32 {
        let Some(at) = self.shelf_named else {
            return 0.0;
        };
        let t = self.now().saturating_sub(at);
        let level = if t < SLOT_NAME_IN_MS {
            t as f32 / SLOT_NAME_IN_MS as f32
        } else if t < SLOT_NAME_IN_MS + SLOT_NAME_HOLD_MS {
            1.0
        } else {
            let out = t - SLOT_NAME_IN_MS - SLOT_NAME_HOLD_MS;
            1.0 - (out as f32 / SLOT_NAME_OUT_MS as f32).min(1.0)
        };
        SLOT_NAME_ALPHA * ease(level)
    }

    fn next_shelf(&self, by: i32) -> Option<usize> {
        let n = self.shelves.len() as i32;
        (1..n)
            .map(|step| (self.shelf_at as i32 + by * step).rem_euclid(n) as usize)
            .find(|at| !self.shelves[*at].1.carts.is_empty())
    }

    fn remembered_seat(&self) -> Option<(usize, usize)> {
        let key = self.state.shelf.as_deref()?;
        let cart = self.state.shelf_cart.as_ref()?;
        let at = self.shelves.iter().position(|(r, _)| r.key() == key)?;
        let i = self.shelves[at]
            .1
            .carts
            .iter()
            .position(|c| c.platform == cart.platform && c.stem == cart.stem)?;
        Some((at, i))
    }

    fn remember_shelf(&mut self) {
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        self.state.shelf_cart = Some(Recent {
            platform: cart.platform,
            stem: cart.stem.clone(),
        });
        self.state.shelf = Some(self.shelves[self.shelf_at].0.key());
        self.persist();
    }

    fn seat_of(&self, stem: &str, platform: Option<Platform>) -> Option<(usize, usize)> {
        self.shelves
            .iter()
            .enumerate()
            .filter(|(_, (r, _))| {
                platform.is_none_or(|want| r.platform().is_none_or(|p| p == want))
            })
            .find_map(|(at, (_, shelf))| {
                shelf
                    .carts
                    .iter()
                    .position(|c| c.stem == stem && platform.is_none_or(|p| c.platform == p))
                    .map(|i| (at, i))
            })
    }

    pub fn confirm_clock(&mut self) {
        let Phase::SetClock {
            picker,
            seed,
            from_menu,
        } = &self.phase
        else {
            return;
        };
        let (moved, offset, from_menu) = (picker.secs() - *seed, picker.offset_min(), *from_menu);
        let utc = self.utc_secs() + moved;
        if let Some(power) = &mut self.power {
            power.set_clock(utc);
        }
        self.state.utc_offset_min = offset as i16;
        self.state.clock_set = true;
        self.persist();
        if from_menu {
            self.phase = Phase::QuickMenu {
                row: QuickRow::DateTime,
            };
        } else {
            self.start();
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn wall_secs(&self) -> i64 {
        self.utc_secs() + i64::from(self.state.utc_offset_min) * 60
    }

    fn utc_secs(&self) -> i64 {
        self.power.as_ref().map_or_else(system_secs, |p| p.now())
    }

    pub fn picker(&self) -> Option<&ClockPicker> {
        match &self.phase {
            Phase::SetClock { picker, .. } => Some(picker),
            _ => None,
        }
    }

    pub fn quick_menu(&self) -> Option<QuickRow> {
        match self.phase {
            Phase::QuickMenu { row } => Some(row),
            _ => None,
        }
    }

    pub fn in_game_menu(&self) -> Option<QuickRow> {
        self.in_game_menu
    }

    pub fn in_game_hidden(&self) -> &'static [QuickRow] {
        let no_bezel = self.playing_platform() == Platform::Gba && !self.framed_panel();
        match (self.palette_live, no_bezel, self.single_cart()) {
            (true, false, false) => &[],
            (true, false, true) => &[QuickRow::ExitGame],
            (true, true, false) => &[QuickRow::Bezel],
            (true, true, true) => &[QuickRow::Bezel, QuickRow::ExitGame],
            (false, false, false) => &[QuickRow::Palette],
            (false, false, true) => &[QuickRow::Palette, QuickRow::ExitGame],
            (false, true, false) => &[QuickRow::Palette, QuickRow::Bezel],
            (false, true, true) => &[QuickRow::Palette, QuickRow::Bezel, QuickRow::ExitGame],
        }
    }

    pub fn gb_palette_label(&self) -> &'static str {
        self.state.gb_palette.label()
    }

    pub fn set_quick_palette_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_palette_faces = Some([dim, lit]);
    }

    fn playing_row(&self, row: QuickRow)…37018 tokens truncated…ant::now() + Duration::from_secs(10));
                    }
                }
            }
        }
        let desired = feedback.unwrap_or(requested);
        let changed = self.link.is_some() && self.observed != Some(desired);
        if cfg!(all(feature = "device", not(test)))
            && self.pending.is_none()
            && (changed || self.next.is_none_or(|at| Instant::now() >= at))
        {
            if let Some(address) = self.address.clone() {
                let link = self.link.clone();
                let generation = self.generation;
                let (tx, rx) = mpsc::channel();
                self.pending = Some(rx);
                std::thread::spawn(move || {
                    let result = sync(&address, link, desired, changed);
                    let _ = tx.send(Reply {
                        generation,
                        requested: desired,
                        result,
                    });
                });
            }
        }
        feedback
    }
}

fn call(destination: &str, path: &str, args: &[&str]) -> Result<String, ()> {
    let output = std::process::Command::new("timeout")
        .args(["2", "dbus-send", "--system", "--print-reply"])
        .arg(format!("--dest={destination}"))
        .arg(path)
        .args(args)
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    String::from_utf8(output.stdout).map_err(|_| ())
}
type Bus = dyn Fn(&str, &str, &[&str]) -> Result<String, ()>;

fn get(
    bus: &Bus,
    destination: &str,
    path: &str,
    interface: &str,
    property: &str,
) -> Result<String, ()> {
    bus(
        destination,
        path,
        &[
            "org.freedesktop.DBus.Properties.Get",
            &format!("string:{interface}"),
            &format!("string:{property}"),
        ],
    )
}
fn set(bus: &Bus, link: &Link, property: &str, value: &str) -> Result<(), ()> {
    bus(
        "org.bluealsa",
        &link.pcm,
        &[
            "org.freedesktop.DBus.Properties.Set",
            "string:org.bluealsa.PCM1",
            &format!("string:{property}"),
            value,
        ],
    )
    .map(|_| ())
}
fn uint16(text: &str) -> Result<u16, ()> {
    let words: Vec<_> = text.split_whitespace().collect();
    words
        .windows(2)
        .find(|w| w[0] == "uint16")
        .and_then(|w| w[1].parse().ok())
        .ok_or(())
}
fn remote(bus: &Bus, link: &Link) -> Result<u8, ()> {
    let value = uint16(&get(
        bus,
        "org.bluez",
        &link.transport,
        "org.bluez.MediaTransport1",
        "Volume",
    )?)?;
    u8::try_from(value).ok().filter(|v| *v <= 127).ok_or(())
}
// Keep Slot's existing squared volume curve, but apply it once, at the headset.
fn native(level: u8) -> u8 {
    ((u32::from(level.min(100)).pow(2) * 127 + 5000) / 10000) as u8
}
fn percent(value: u8) -> u8 {
    ((f32::from(value) / 127.0).sqrt() * 100.0).round() as u8
}
fn transport(text: &str, address: &str) -> Option<String> {
    let suffix = format!("/dev_{}/", address.replace(':', "_").to_ascii_uppercase());
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("object path \""))
        .filter_map(|line| line.strip_suffix('"'))
        .find(|path| {
            path.starts_with("/org/bluez/hci")
                && path.contains(&suffix)
                && path.rsplit('/').next().is_some_and(|s| {
                    s.starts_with("fd") && s[2..].bytes().all(|b| b.is_ascii_digit())
                })
        })
        .map(str::to_owned)
}
fn discover(bus: &Bus, address: &str) -> Result<Link, ()> {
    if !crate::bluetooth::valid_address(address) {
        return Err(());
    }
    let objects = bus(
        "org.bluez",
        "/",
        &["org.freedesktop.DBus.ObjectManager.GetManagedObjects"],
    )?;
    let transport = transport(&objects, address).ok_or(())?;
    let device = transport.rsplit_once('/').ok_or(())?.0;
    let pcm = format!(
        "{}/a2dpsrc/sink",
        device.replacen("/org/bluez/", "/org/bluealsa/", 1)
    );
    Ok(Link { pcm, transport })
}
fn write(bus: &Bus, link: &Link, value: u8) -> Result<u8, ()> {
    let packed = u16::from(value) * 257;
    set(bus, link, "Volume", &format!("variant:uint16:{packed}"))?;
    // BlueALSA can swallow a failed native write. Require BlueZ readback before
    // removing software attenuation. One native step of rounding is harmless.
    let actual = remote(bus, link)?;
    if actual.abs_diff(value) > 1 {
        return Err(());
    }
    Ok(actual)
}
fn sync(address: &str, existing: Option<Link>, level: u8, changed: bool) -> Result<(Link, u8), ()> {
    sync_with(&call, address, existing, level, changed)
}
fn sync_with(
    bus: &Bus,
    address: &str,
    existing: Option<Link>,
    level: u8,
    changed: bool,
) -> Result<(Link, u8), ()> {
    let first = existing.is_none();
    let link = match existing {
        Some(link) => link,
        None => discover(bus, address)?,
    };
    let result = (|| {
        let current = remote(bus, &link)?; // Optional property: absence means no native support.
        let value = if first {
            let soft = get(
                bus,
                "org.bluealsa",
                &link.pcm,
                "org.bluealsa.PCM1",
                "SoftVolume",
            )?;
            let target = if soft.split_whitespace().last() == Some("true") {
                // Migration: preserve both existing attenuation stages instead of
                // suddenly removing Slot's software gain or maxing the headset.
                ((u32::from(current) * u32::from(level).pow(2)) / 10000) as u8
            } else {
                current.min(native(level))
            };
            set(bus, &link, "SoftVolume", "variant:boolean:false")?;
            write(bus, &link, target)?
        } else if changed {
            write(bus, &link, native(level))?
        } else {
            current
        };
        Ok((link.clone(), percent(value)))
    })();
    if result.is_err() {
        // Fall back to Slot's original software gain. Never raise remote volume.
        if set(bus, &link, "SoftVolume", "variant:boolean:true").is_ok() {
            let _ = set(bus, &link, "Volume", "variant:uint16:32639");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scripted(
        steps: Vec<(&'static str, Result<&'static str, ()>)>,
    ) -> impl Fn(&str, &str, &[&str]) -> Result<String, ()> {
        let steps = std::cell::RefCell::new(std::collections::VecDeque::from(steps));
        move |_, _, args| {
            let (expected, reply) = steps
                .borrow_mut()
                .pop_front()
                .expect("unexpected D-Bus request");
            assert!(
                args.join(" ").contains(expected),
                "{args:?} should contain {expected}"
            );
            reply.map(str::to_owned)
        }
    }
    const OBJECTS: &str = "object path \"/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF/fd0\"";
    #[test]
    fn migration_preserves_existing_attenuation() {
        let bus = scripted(vec![
            ("GetManagedObjects", Ok(OBJECTS)),
            ("string:Volume", Ok("variant uint16 64")),
            ("string:SoftVolume", Ok("variant boolean true")),
            ("variant:boolean:false", Ok("")),
            ("variant:uint16:4112", Ok("")), // remote 64 * Slot 50%^2 = 16
            ("string:Volume", Ok("variant uint16 16")),
        ]);
        assert_eq!(
            sync_with(&bus, "AA:BB:CC:DD:EE:FF", None, 50, false)
                .unwrap()
                .1,
            percent(16)
        );
    }
    #[test]
    fn reconnect_does_not_apply_software_attenuation_again() {
        let bus = scripted(vec![
            ("GetManagedObjects", Ok(OBJECTS)),
            ("string:Volume", Ok("variant uint16 64")),
            ("string:SoftVolume", Ok("variant boolean false")),
            ("variant:boolean:false", Ok("")),
            ("variant:uint16:16448", Ok("")),
            ("string:Volume", Ok("variant uint16 64")),
        ]);
        assert_eq!(
            sync_with(&bus, "AA:BB:CC:DD:EE:FF", None, 80, false)
                .unwrap()
                .1,
            percent(64)
        );
    }
    #[test]
    fn native_write_must_be_confirmed_before_bypassing_software() {
        let bus = scripted(vec![
            ("string:Volume", Ok("variant uint16 64")),
            ("variant:uint16:8224", Ok("")), // desired 50% = 32
            ("string:Volume", Ok("variant uint16 64")), // device ignored write
            ("variant:boolean:true", Ok("")),
            ("variant:uint16:32639", Ok("")), // software unity, no remote change
        ]);
        assert!(sync_with(&bus, "unused", linked().link, 50, true).is_err());
    }
    #[test]
    fn headphone_buttons_are_read_without_writing_back() {
        let bus = scripted(vec![("string:Volume", Ok("variant uint16 90"))]);
        assert_eq!(
            sync_with(&bus, "unused", linked().link, 50, false)
                .unwrap()
                .1,
            percent(90)
        );
    }
    #[test]
    fn unsupported_transport_retains_software_control() {
        let bus = scripted(vec![
            ("GetManagedObjects", Ok(OBJECTS)),
            ("string:Volume", Err(())),
            ("variant:boolean:true", Ok("")),
            ("variant:uint16:32639", Ok("")),
        ]);
        assert!(sync_with(&bus, "AA:BB:CC:DD:EE:FF", None, 50, false).is_err());
    }
    #[test]
    fn native_curve_and_roundtrip() {
        assert_eq!(native(0), 0);
        assert_eq!(native(100), 127);
        assert_eq!(native(50), 32);
        for value in 0..=127 {
            assert!(native(percent(value)).abs_diff(value) <= 1);
        }
    }
    #[test]
    fn transport_is_selected_device_only() {
        let text = "object path \"/org/bluez/hci1/dev_AA_BB_CC_DD_EE_FF\"\nobject path \"/org/bluez/hci1/dev_AA_BB_CC_DD_EE_FF/fd2\"";
        assert_eq!(
            transport(text, "AA:BB:CC:DD:EE:FF").unwrap(),
            "/org/bluez/hci1/dev_AA_BB_CC_DD_EE_FF/fd2"
        );
        assert!(transport(text, "00:11:22:33:44:55").is_none());
        assert_eq!(uint16("variant uint16 63"), Ok(63));
        assert!(uint16("variant boolean true").is_err());
    }
    fn linked() -> Volume {
        Volume {
            address: Some("AA:BB:CC:DD:EE:FF".into()),
            link: Some(Link {
                pcm: "pcm".into(),
                transport: "transport".into(),
            }),
            observed: Some(50),
            ..Default::default()
        }
    }
    #[test]
    fn mute_and_other_outputs_never_bypass_software() {
        let v = linked();
        assert_eq!(v.software_volume(v.address.as_deref(), 50), 100);
        assert_eq!(v.software_volume(v.address.as_deref(), 0), 0);
        assert_eq!(v.software_volume(None, 20), 20);
        assert_eq!(v.software_volume(Some("other"), 20), 20);
    }
    #[test]
    fn newer_button_press_wins_over_remote_feedback() {
        let mut v = linked();
        let (tx, rx) = mpsc::channel();
        tx.send(Reply {
            generation: 0,
            requested: 50,
            result: Ok((v.link.clone().unwrap(), 40)),
        })
        .unwrap();
        v.pending = Some(rx);
        assert_eq!(v.poll(Some("AA:BB:CC:DD:EE:FF"), 60), None);
    }
    #[test]
    fn stale_route_reply_is_discarded() {
        let mut v = linked();
        let (tx, rx) = mpsc::channel();
        tx.send(Reply {
            generation: 0,
            requested: 50,
            result: Ok((v.link.clone().unwrap(), 90)),
        })
        .unwrap();
        v.pending = Some(rx);
        assert_eq!(v.poll(None, 20), None);
        assert!(v.link.is_none());
        assert_eq!(v.software_volume(None, 20), 20);
    }
}
