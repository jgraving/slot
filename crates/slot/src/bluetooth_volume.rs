//! Optional AVRCP volume through BlueALSA. Never write BlueZ's transport directly:
//! only its owner (BlueALSA) is allowed to do that.
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct Link {
    pcm: String,
    transport: String,
}
struct Reply {
    generation: u64,
    requested: u8,
    result: Result<(Link, u8), ()>,
}
#[derive(Default)]
pub struct Volume {
    address: Option<String>,
    generation: u64,
    link: Option<Link>,
    observed: Option<u8>,
    pending: Option<Receiver<Reply>>,
    next: Option<Instant>,
}
impl Volume {
    pub fn software_volume(&self, address: Option<&str>, volume: u8) -> u8 {
        if volume != 0 && address == self.address.as_deref() && self.link.is_some() {
            100
        } else {
            volume
        }
    }

    // One bounded worker at a time; late replies never affect a new route, and
    // a remote observation never overwrites a newer press of the volume buttons.
    pub fn poll(&mut self, address: Option<&str>, requested: u8) -> Option<u8> {
        if address != self.address.as_deref() {
            self.address = address.map(str::to_owned);
            self.generation = self.generation.wrapping_add(1);
            self.link = None;
            self.observed = None;
            self.next = None;
        }
        let mut feedback = None;
        if let Some(reply) = self.pending.as_ref().and_then(|r| r.try_recv().ok()) {
            self.pending = None;
            if reply.generation == self.generation {
                match reply.result {
                    Ok((link, level)) => {
                        self.link = Some(link);
                        self.observed = Some(level);
                        if requested == reply.requested {
                            feedback = Some(level);
                        }
                        self.next = Some(Instant::now() + Duration::from_millis(750));
                    }
                    Err(()) => {
                        self.link = None;
                        self.observed = None;
                        self.next = Some(Instant::now() + Duration::from_secs(10));
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
