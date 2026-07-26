# gog-source-wasm-plugin

A `SourcePlugin` for [Concourse](https://github.com/smh0505/Concourse) implemented as a WASM
component, ported from that project's built-in `gog.rs` +
`src/plugins/gog/index.ts` (both since deleted from the host app now that this plugin is the
sole GOG implementation). Enumerates installed GOG games from the registry
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

Unlike Steam/Epic, `launch()` here is real, not dead code: GOG has no OS-registered URI scheme
that launches a specific installed game (it does register `goggalaxy://`, confirmed via a
real registry check, but that's not documented or used by any known reference implementation
for launching a specific game by id - even community wrappers use the same direct CLI
invocation this does). `launch()` resolves `GalaxyClient.exe`'s real path via the registry
each call (`HKLM\...\GalaxyClient\paths`, 64-bit location tried first) and spawns it with
`/gameid <id> /command runGame` - the host app's `library.ts` calls this plugin's own
`launch()` directly for `"gog://"`-prefixed entries, the same way any other `SourcePlugin`'s
`launch()` would be used. Verified against a real installed GOG game - `GalaxyClient.exe`
launches for real, not just "no error returned."

## Building

```sh
rustup target add wasm32-wasip1   # once
cargo install cargo-component     # once
cargo component build
```

Output: `target/wasm32-wasip1/debug/gog_source_wasm_plugin.wasm`.

## Installing into a running Concourse

Copy the compiled `.wasm` and `plugin.json` into
`<app data dir>/wasm-plugins/gog-wasm/` (Windows:
`%APPDATA%\com.bloppy.concourse\wasm-plugins\gog-wasm\`). It'll show up in Settings' Plugins
panel next time the app starts, as **GOG (WASM)**.

## Versioning

Plain SemVer (`Cargo.toml` + `plugin.json`'s `version`), independent of Concourse's own
milestone-tracked version - patch for fixes, minor for backward-compatible new capabilities,
major for breaking manifest/WIT interface changes. Full convention:
`.claude/CLAUDE.md` (Plugin Versioning) in the main `concourse` repo.
