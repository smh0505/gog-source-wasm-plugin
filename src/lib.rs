//! WASM `SourcePlugin` for GOG, ported from the game-library-client's built-in `gog.rs` +
//! `src/plugins/gog/index.ts`. Registry access goes through the `host` interface instead of
//! `winreg` directly, since guest code is sandboxed - including `list-registry-keys`, added
//! to the shared host interface specifically for this port (GOG needs to enumerate installed
//! game IDs as registry subkeys, which the Steam port never needed - it only ever read
//! single known-name values).
//!
//! Unlike Steam/Epic, `launch()` here is real, not dead code: GOG has no OS-registered URI
//! scheme of its own that the host could dispatch generically via `openUrl()` (`GalaxyClient.exe`
//! must be invoked directly with `/gameid <id> /command runGame`), so `library.ts` calls this
//! plugin's own `launch()` export directly for `"gog://"`-prefixed entries, the same way any
//! other `SourcePlugin` would be used. The GalaxyClient.exe path itself still has to be
//! resolved via the registry each call - it's not something `install()`-style logic would
//! help with, GOG Galaxy manages its own install location entirely outside this app.

#[allow(warnings)]
mod bindings;

use bindings::exports::gamelib::plugin::source_plugin::{GameEntry, Guest};
use bindings::gamelib::plugin::host;

struct GogPlugin;

struct GogApp {
    game_id: String,
    name: String,
    install_dir: String,
}

/// Tries the 64-bit registry location first, then the 32-bit one - mirrors the built-in
/// `gog.rs`'s `gog_apps_from_registry`.
fn find_gog_apps() -> Vec<GogApp> {
    for games_key in [
        "SOFTWARE\\WOW6432Node\\GOG.com\\Games",
        "SOFTWARE\\GOG.com\\Games",
    ] {
        let Ok(game_ids) = host::list_registry_keys("HKLM", games_key) else {
            continue;
        };

        let mut apps = Vec::new();
        for game_id in game_ids {
            let key_path = format!("{}\\{}", games_key, game_id);
            let name = host::read_registry_string("HKLM", &key_path, "gameName");
            let path = host::read_registry_string("HKLM", &key_path, "path");
            if let (Some(name), Some(path)) = (name, path) {
                apps.push(GogApp {
                    game_id,
                    name,
                    install_dir: path,
                });
            }
        }

        if !apps.is_empty() {
            return apps;
        }
    }

    Vec::new()
}

fn to_game_entry(app: &GogApp) -> GameEntry {
    GameEntry {
        id: format!("gog-{}", app.game_id),
        title: app.name.clone(),
        // GOG has no registered URI scheme GalaxyClient.exe launches specific games through -
        // it does register goggalaxy:// (confirmed via a real registry check), but that's not
        // documented to support launching a specific already-installed game by id, and no
        // known reference implementation uses it for that; this pseudo-URI just exists so
        // library.ts's generic dispatch routes to this plugin's own launch() below instead of
        // openUrl(), same as how it recognizes "steam://"/Epic's real URI scheme.
        executable_path: format!("gog://{}", app.game_id),
        platform: "gog".to_string(),
        cover_art_url: None,
        install_dir: Some(app.install_dir.clone()),
    }
}

/// Tries the 64-bit registry location first, then the 32-bit one - mirrors the built-in
/// `gog.rs`'s (now-retired) `gog_galaxy_client_dir_from_registry`.
fn galaxy_client_path() -> Result<String, String> {
    for subkey in [
        "SOFTWARE\\WOW6432Node\\GOG.com\\GalaxyClient\\paths",
        "SOFTWARE\\GOG.com\\GalaxyClient\\paths",
    ] {
        if let Some(client_dir) = host::read_registry_string("HKLM", subkey, "client") {
            return Ok(format!("{}\\GalaxyClient.exe", client_dir.trim_end_matches('\\')));
        }
    }
    Err("GOG Galaxy installation not found".to_string())
}

impl Guest for GogPlugin {
    fn scan() -> Result<Vec<GameEntry>, String> {
        Ok(find_gog_apps().iter().map(to_game_entry).collect())
    }

    /// GalaxyClient relays the launch to the actual game process and returns quickly, so
    /// (like Steam/Epic URI launches) there's no child process handle worth waiting on -
    /// playtime tracking is intentionally skipped here, same as the other launcher-owned
    /// sources (folder-based tracking covers it instead).
    fn launch(entry: GameEntry) -> Result<(), String> {
        let game_id = entry.id.strip_prefix("gog-").unwrap_or(&entry.id);
        let client_path = galaxy_client_path()?;
        host::spawn_process(
            &client_path,
            &[
                "/gameid".to_string(),
                game_id.to_string(),
                "/command".to_string(),
                "runGame".to_string(),
            ],
        )
    }

    fn get_install_status(entry: GameEntry) -> Result<bool, String> {
        Ok(find_gog_apps()
            .iter()
            .any(|app| format!("gog-{}", app.game_id) == entry.id))
    }
}

bindings::export!(GogPlugin with_types_in bindings);
