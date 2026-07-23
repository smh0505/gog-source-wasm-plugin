//! WASM `SourcePlugin` for GOG, ported from the game-library-client's built-in `gog.rs` +
//! `src/plugins/gog/index.ts`. Registry access goes through the `host` interface instead of
//! `winreg` directly, since guest code is sandboxed - including `list-registry-keys`, added
//! to the shared host interface specifically for this port (GOG needs to enumerate installed
//! game IDs as registry subkeys, which the Steam port never needed - it only ever read
//! single known-name values).
//!
//! `launch()` is implemented for contract-completeness but is dead code in practice, same
//! reasoning as the Steam port: the host app's `library.ts` dispatches `"gog://"`-prefixed
//! `executable_path` values itself via `invoke("launch_gog_game", ...)`, never through a
//! plugin's own `launch()` export. There's no host primitive that replicates
//! `GalaxyClient.exe /gameid <id> /command runGame`'s argument shape beyond generic
//! `spawn-process`, so this best-effort mirrors that CLI form directly.

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
        // GOG has no registered URI scheme - GalaxyClient.exe is invoked directly with CLI
        // flags, so this pseudo-URI only exists to route the host app's generic launch
        // dispatch (library.ts) to invoke("launch_gog_game", ...) rather than openUrl().
        executable_path: format!("gog://{}", app.game_id),
        platform: "gog".to_string(),
        cover_art_url: None,
        install_dir: Some(app.install_dir.clone()),
    }
}

impl Guest for GogPlugin {
    fn scan() -> Result<Vec<GameEntry>, String> {
        Ok(find_gog_apps().iter().map(to_game_entry).collect())
    }

    fn launch(entry: GameEntry) -> Result<(), String> {
        let game_id = entry.id.strip_prefix("gog-").unwrap_or(&entry.id);
        host::spawn_process(
            "GalaxyClient.exe",
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
