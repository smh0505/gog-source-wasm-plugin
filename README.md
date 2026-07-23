# gog-source-wasm-plugin

A `SourcePlugin` for Game Library Client implemented as a WASM
component (Milestone 8), ported from that project's built-in `gog.rs` +
`src/plugins/gog/index.ts`. Enumerates installed GOG games from the registry
(`HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\{gameID}`), same logic as the built-in version -
just running sandboxed via `wasmtime` instead of compiled directly into the host app.

This is a real, separate repo on purpose - same reasoning as `steam-source-wasm-plugin`: a
plugin whose source lives inside the host app's own repo doesn't genuinely exercise the
"install arbitrary third-party code" model the WASM plugin system is for.

This port needed one new host primitive that didn't exist yet when `steam-source-wasm-plugin`
was built: `list-registry-keys` (enumerate registry *subkey names*, not just read a single
known value). GOG stores each installed game as its own subkey under `...\GOG.com\Games\`,
unlike Steam, which only ever needed single named-value reads. Added to the shared
`wit/plugin.wit` host interface and implemented in the host app's `wasm_plugins.rs`.

`launch()` is implemented for contract-completeness but is dead code in practice - the host
app's generic launch dispatch handles `gog://` pseudo-URIs directly off the stored
`executable_path` (routing to `invoke("launch_gog_game", ...)`), never actually calling a
plugin's own `launch()` export. There's no host primitive that replicates
`GalaxyClient.exe /gameid <id> /command runGame`'s exact argument shape beyond generic
`spawn-process`, which this uses as a best-effort mirror.

## Building

```sh
rustup target add wasm32-wasip1   # once
cargo install cargo-component     # once
cargo component build
```

Output: `target/wasm32-wasip1/debug/gog_source_wasm_plugin.wasm`.

## Installing into a running Game Library Client

Copy the compiled `.wasm` and `plugin.json` into
`<app data dir>/wasm-plugins/gog-wasm/` (Windows:
`%APPDATA%\com.minho.tauri-app\wasm-plugins\gog-wasm\`). It'll show up in Settings
alongside the built-in GOG plugin next time the app starts, as **GOG (WASM)** -
deliberately a different id/name (`gog-wasm`, not `gog`) so it doesn't collide with the
built-in one while both exist side by side for comparison.
