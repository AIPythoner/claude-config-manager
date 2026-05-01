# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
npm install              # install frontend deps (Rust deps fetched on first build)
npm run tauri dev        # run app with hot-reload (Vite + Tauri window)
npm run tauri build      # build NSIS installer at src-tauri/target/release/bundle/nsis/
npm run build            # frontend-only: tsc + vite build (rarely run alone)
```

There are no tests and no linter wired up. Vite dev server runs on port **5174** (not the Tauri default 1420) — `tauri.conf.json` and `vite.config.ts` must agree on this port.

The window is fixed 380×520 non-resizable on every platform. `bundle.targets = "all"` so each OS builds its native installer (NSIS on Windows, dmg on macOS, deb+AppImage on Linux). The release flow is `.github/workflows/release.yml` — triggered by pushing a `v*` tag (or manual `workflow_dispatch`); it runs four matrix jobs and publishes a draft GitHub Release. `release-hybrid-example.yml` next to it is an unrelated Python+Tauri example, not used. When cutting a release, bump the version in `package.json`, `src-tauri/Cargo.toml`, **and** `src-tauri/tauri.conf.json` (all three must agree) before tagging.

## Architecture

This is a Tauri 2 desktop app whose only purpose is to **read/write the config files of other CLI tools** (Claude Code, Gemini CLI, Codex CLI, OpenCode). Understanding the merge-vs-overwrite semantics is the central thing to get right.

### Two storage layers

1. **App's own config** — `%APPDATA%/claude-config-manager/configs.json`, a single `ConfigStore { configs: Vec<Config> }`. Each `Config` carries `config_type` (Claude/Gemini/Codex), `api_key`, `base_url`, `model`, and `is_active`. Loaded/saved via `load_store` / `save_store` in `src-tauri/src/lib.rs`.
2. **Target tool config files** — written into the user's home directory at activation time. `~/.claude/settings.json`, `~/.gemini/.env`, `~/.codex/{auth.json,config.toml}`, `~/.config/opencode/opencode.json`.

Activation = "make this stored config the live one" by editing the target tool's file. Deactivation/deletion = remove only the keys we own.

### Critical invariant: merge, don't overwrite

When writing to a target tool's config, we only touch the keys we manage and preserve everything else the user has set:

- **Claude** (`update_claude_env` in `lib.rs`): parses `settings.json` as JSON, mutates only `env.ANTHROPIC_AUTH_TOKEN`, `env.ANTHROPIC_BASE_URL`, `env.ANTHROPIC_MODEL`. Other fields (`permissions`, `statusLine`, `enabledPlugins`, …) must stay intact.
- **Gemini** (`update_gemini_env`): line-based filter on `.env`, drops only the three target keys (`GEMINI_API_KEY`, `GOOGLE_GEMINI_BASE_URL`, `GEMINI_MODEL`), keeps comments and other lines, then appends fresh values. If the file would become empty, delete it.
- **Codex** (`apply_codex_config`): rewrites `auth.json` and `config.toml` wholesale — these files are considered owned by this app. The `config.toml` includes a hardcoded `model_provider = "fox"` template.
- **OpenCode** (`apply_opencode_config`): reads existing `opencode.json` if present (else uses the embedded `get_opencode_template`), then merges `apiKey` / `baseURL` into the matching provider blocks (`foxcode-claude` / `foxcode-gemini` / `foxcode-oai`) and updates the top-level `model` to the last selected one.

If you add a new target-tool integration, follow the same pattern: read → mutate only owned keys → write. Never use `serde_json::to_string` of a freshly-built struct as the file content unless that file is fully app-owned.

### Type-scoped activation

`activate_config` only deactivates other configs **of the same `config_type`** — Claude/Gemini/Codex can each have one active simultaneously. OpenCode is not a `ConfigType`; it's a derived view that pulls from already-saved Claude/Gemini/Codex entries via dropdowns, so it does not get its own "active" flag.

`restore_claude_login` is a special path that clears the Claude env keys to fall back to Anthropic's official OAuth login (the keys we wrote would otherwise force third-party auth).

### Frontend ↔ backend conventions

- All backend logic lives in `src-tauri/src/lib.rs`. `main.rs` is a 5-line entry point. Commands are registered in the `invoke_handler!` macro at the bottom of `lib.rs`.
- The frontend is a **single `src/main.ts`** with no framework — it builds HTML strings, sets `app.innerHTML`, and uses inline `onclick="..."` handlers. Functions called from inline handlers must be assigned to `window` (search for `(window as any).` assignments at the bottom of `main.ts`). CSP is set to `null` in `tauri.conf.json` to allow this.
- **Parameter name casing** is the most common foot-gun: Rust command params are `snake_case` (`api_key`, `base_url`, `config_type`), but `invoke()` calls from TS must pass them as **camelCase** (`apiKey`, `baseUrl`, `configType`) — Tauri does the conversion. Mismatches fail silently as "missing field" errors.
- Adding a new Tauri command: define `#[tauri::command] fn ...`, register it in the `invoke_handler!` list, and call from TS with camelCased args.

### Adding a new ConfigType

If a new AI tool needs supporting, the touch-points are:
1. `ConfigType` enum + the `match` arms in `add_config`, `apply_config`, `clear_config`.
2. A new `apply_<tool>_config` / `clear_<tool>_config` pair following the merge-don't-overwrite rule.
3. Frontend: add to `ConfigType` union, `CONFIG_TYPE_LABELS`, `CONFIG_TYPE_COLORS`, the tab list in `renderConfigs`, and the `getKeyLabel` / `getUrlLabel` switches.
