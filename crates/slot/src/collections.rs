//! Device-local shelf membership. Cartridge files and Cart Studio formats are untouched.
use slot_store::{atomic_write, Cart, Platform};
use std::io;
use std::path::Path;

pub const RECENT_LIMIT: usize = 25;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CartKey {
    pub platform: Platform,
    pub stem: String,
}
impl CartKey {
    pub fn of(cart: &Cart) -> Self {
        Self {
            platform: cart.platform,
            stem: cart.stem.clone(),
        }
    }
    fn parse(line: &str) -> Option<Self> {
        let (system, stem) = line.split_once('/')?;
        let platform = Platform::ALL
            .into_iter()
            .find(|p| p.dir_name().eq_ignore_ascii_case(system))?;
        if stem.is_empty()
            || stem == "."
            || stem == ".."
            || stem.contains(['/', '\\', '\0', '\r', '\n'])
        {
            return None;
        }
        Some(Self {
            platform,
            stem: stem.into(),
        })
    }
}
#[derive(Clone, Default)]
pub struct Collections {
    pub favorites: Vec<CartKey>,
    pub recent: Vec<CartKey>,
}
impl Collections {
    pub fn read(root: &Path) -> io::Result<Self> {
        Ok(Self {
            favorites: read_list(root, "favourites")?,
            recent: read_list(root, "recent")?
                .into_iter()
                .take(RECENT_LIMIT)
                .collect(),
        })
    }
    pub fn is_favorite(&self, cart: &Cart) -> bool {
        self.favorites.contains(&CartKey::of(cart))
    }
    pub fn toggle(&mut self, cart: &Cart) -> bool {
        let key = CartKey::of(cart);
        if let Some(i) = self.favorites.iter().position(|k| *k == key) {
            self.favorites.remove(i);
            false
        } else {
            self.favorites.push(key);
            true
        }
    }
    pub fn launched(&mut self, cart: &Cart) {
        let key = CartKey::of(cart);
        self.recent.retain(|k| *k != key);
        self.recent.insert(0, key);
        self.recent.truncate(RECENT_LIMIT);
    }
    pub fn save_favorites(&self, root: &Path) -> io::Result<()> {
        write_list(root, "favourites", &self.favorites)
    }
    pub fn save_recent(&self, root: &Path) -> io::Result<()> {
        write_list(root, "recent", &self.recent)
    }
    pub fn shelves(&self, carts: &[Cart]) -> (Vec<Cart>, Vec<Cart>) {
        let resolve = |keys: &[CartKey]| -> Vec<Cart> {
            keys.iter()
                .filter_map(|k| carts.iter().find(|c| CartKey::of(c) == *k).cloned())
                .collect()
        };
        let mut favorites = resolve(&self.favorites);
        favorites.sort_by_key(|c| (slot_store::sort_key(&c.stem), c.platform as u8));
        (resolve(&self.recent), favorites)
    }
}
fn read_list(root: &Path, file: &str) -> io::Result<Vec<CartKey>> {
    let text = match std::fs::read_to_string(root.join("Config").join(file)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut keys = Vec::new();
    for key in text.lines().filter_map(CartKey::parse) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Ok(keys)
}
fn write_list(root: &Path, file: &str, keys: &[CartKey]) -> io::Result<()> {
    let dir = root.join("Config");
    std::fs::create_dir_all(&dir)?;
    let text: String = keys
        .iter()
        .map(|k| {
            format!(
                "{}/{}\n",
                k.platform.dir_name().to_ascii_lowercase(),
                k.stem
            )
        })
        .collect();
    atomic_write(&dir.join(file), text.as_bytes())
}
