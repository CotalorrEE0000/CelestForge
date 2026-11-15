# CelestForge

A visual novel (galgame) engine written from scratch in Rust. JSON scripts in, GPU-rendered scenes out — no Ren'Py, no KiriKiri, no game framework on top of `wgpu`.

Script parsing, the virtual machine, the renderer and the player are all hand-written.

---

## Requirements

| Item | Notes |
|---|---|
| Rust | 1.85+ (the workspace uses edition 2024) |
| GPU | Anything `wgpu` 22 can drive: Vulkan, DX12 or Metal |
| `assets/font.ttf` | Loaded at startup; the player panics if it is missing |

---

## Build and run

```bash
git clone https://github.com/CotalorrEE0000/CelestForge.git
cd CelestForge
cargo run -p celest_player
```

The player loads its script from a fixed path: **`scripts/test_chapter1.json`**, relative to the working directory. The directory is not tracked by git, so create it yourself:

```bash
mkdir -p scripts
```

Then save a script there. The smallest one that runs:

```json
[
  { "type": "Log", "text": "CelestForge started" },
  { "type": "ChapterTitle", "title": "Prologue", "duration_ms": 2000 },
  { "type": "SetBg", "asset_id": "assets/bg/bg_room.png" },
  { "type": "Text", "speaker": "Narrator", "content": "The room is quiet." },
  { "type": "ShowSprite", "who": "hiro", "asset_id": "assets/sprite/char_hiro_idle.png", "position": "Center" },
  { "type": "Text", "speaker": "Hiro", "content": "What lies beyond that sky?" },
  { "type": "End" }
]
```

A terminal-only player is also included, useful for checking a script without a GPU:

```bash
cargo run -p celest_player --bin cli_play
```

### Files on disk

| Path | Purpose |
|---|---|
| `assets/bg/`, `assets/sprite/` | Background and sprite images, referenced by `asset_id` |
| `assets/font.ttf` | UI font (CJK included) |
| `scripts/test_chapter1.json` | The script the player loads |
| `saves/` | Created on first save: `quick.json` for quick save, `slot_1.json` … `slot_6.json` for the menu slots |

`asset_id` is a path relative to the working directory, so `assets/bg/bg_room.png` is the canonical form.

### Controls

| Key | Action |
|---|---|
| `Space` / `Enter` | Advance dialogue |
| `Esc` | System menu (in game), back out (in submenus) |
| `Tab` | Toggle autoplay |
| `F5` / `F9` | Quick save / quick load |
| `←` / `PageUp` | Open backlog |
| `→` / `PageDown` | Close backlog |

Quick load reads `saves/quick.json` unconditionally, so quick save at least once before pressing `F9`.

---

## Project layout

```
CelestForge/
├── Cargo.toml               workspace manifest, three crates
├── assets/                  font, backgrounds, sprites
├── scripts/                 scripts to run (untracked)
├── saves/                   save files (untracked, created on demand)
│
├── core/                    celest_core — pure logic, no rendering, no window
│   └── src/
│       ├── command.rs       Command enum: the entire script vocabulary
│       ├── loader.rs        JSON → Command, label table, ending fallback
│       ├── vm.rs            step(), VmWait, jumps, timers
│       └── state.rs         flags, vars, backlog history
│
├── renderer/                celest_renderer — wgpu only, no UI
│   └── src/
│       ├── lib.rs           scene pass, draw queue, window surface
│       ├── pipeline.rs      render pipeline and uniform layout
│       ├── texture_cache.rs texture upload, keyed by asset id
│       ├── sprite.rs        quad geometry for bg and sprites
│       └── vertex.rs
│
└── player/                  celest_player — winit + egui, binds everything
    └── src/
        ├── main.rs          application handler, frame loop, egui wiring
        ├── game.rs          GameSession: owns the VM, executes side effects
        ├── ui.rs            title / dialogue / menu / slots / backlog / settings
        ├── input.rs         key event → InputAction
        ├── save.rs          SaveData ⇄ saves/*.json
        └── bin/cli_play.rs  terminal player
```

The layering rule is strict: `core` knows nothing about rendering, `renderer` knows nothing about egui, `ui.rs` knows nothing about wgpu, and only `main.rs` binds them together.

| Crate | Responsibility |
|---|---|
| `celest_core` | Command set, VM, script loader, game state |
| `celest_renderer` | wgpu pipeline, texture cache, quad batching |
| `celest_player` | winit window, egui screens, input mapping, saves |

---

## Script format

A script is a JSON array. Every element is an object with a `"type"` field; the loader rejects unknown types and missing fields with the index of the offending command, so a broken script fails at startup instead of silently halfway through a scene.

### Commands

| `type` | Fields | Behaviour |
|---|---|---|
| `Text` | `content`, `speaker` (optional) | Shows a line, waits for a click |
| `ChapterTitle` | `title`, `duration_ms` | Full-screen title, fades in and out, then continues |
| `Wait` | — | Waits for a click without showing text |
| `WaitMs` | `ms` | Waits `ms` milliseconds |
| `SetBg` | `asset_id` | Sets the background |
| `ClearBg` | — | Removes the background |
| `ShowSprite` | `who`, `asset_id`, `position` | Draws a sprite under the given name |
| `HideSprite` | `who` | Removes that sprite |
| `ClearSprite` | — | Removes all sprites |
| `Choice` | `options[]` | Branches; each option is `text`, `jump_to`, `condition` (optional flag) |
| `Label` | `name` | Jump target |
| `Jump` | `dest` | Unconditional jump to a label |
| `JumpIf` | `flag`, `dest` | Jumps when the flag is set, otherwise falls through |
| `SetFlag` | `flag`, `value` | Sets a boolean flag |
| `ClearFlag` | `name` | Clears a flag |
| `SetVar` | `name`, `value` | Sets an integer variable |
| `Log` | `text` | Writes a line to stdout and to the backlog |
| `End` | — | Ends the script; the player returns to the title screen |

`position` is `"Left"`, `"Center"`, `"Right"`, or `"Absolute"` with extra `x`, `y` fields:

```json
{ "type": "ShowSprite", "who": "hiro", "asset_id": "assets/sprite/char_hiro_idle.png",
  "position": "Absolute", "x": 0.7, "y": 0.1 }
```

A `Choice` option is only shown when its `condition` flag is set; options without a condition are always shown.

```json
{ "type": "Choice", "options": [
    { "text": "Ask about the sky", "jump_to": "ask_sky" },
    { "text": "Mention the letter", "jump_to": "ask_letter", "condition": "saw_letter" }
]}
```

Parsed but currently inert (accepted by the loader, does nothing at runtime): `MoveSprite`, `Transition`, `Movie`, `PlayBgm`, `StopBgm`, `PlaySe`, `StopSe`, `PlayVoice`, `StopVoice`. Audio commands print a line to stderr and are skipped.

### Two things the script does not have to get right

- **The end.** If a script has no `End`, the loader appends a closing `ChapterTitle` and an `End` after it. If it has an `End` but the last real command before it is not a `ChapterTitle`, one is inserted. Without this the VM would run past the last command, keep reporting "running", and freeze the window forever.
- **Nothing else.** Jumps are not validated: a `dest` that matches no `Label` ends the script instead of jumping. Typos in asset paths fail at texture load time.

### Save data

A save file stores the VM program counter, flags, variables, the current draw queue and a text preview. The counter is saved as `pc - 1` because the VM advances past a command before suspending on it; loading re-executes that command and suspends again naturally.

---

## Implementation status

Working: script loading and validation, the command VM, backgrounds, sprites with named slots, dialogue, choice branching, flags and variables, quick save/load, six save slots with overwrite confirmation, title screen, system menu, settings screen, backlog (last 100 lines), autoplay, chapter titles, Chinese and English text.

Parsed but inert: everything in the audio group above, sprite movement, transitions and movies.

Not started: read-text tracking and skip.

---

## 中文简介

CelestForge 是一个用 Rust 从零手写的视觉小说引擎，不依赖 Ren'Py 或任何现成游戏框架。

- **三层 crate**：`celest_core`（指令集 + VM + 脚本加载器，纯逻辑）、`celest_renderer`（wgpu 渲染）、`celest_player`（winit 窗口 + egui 界面 + 存档）。
- **脚本即 JSON**：数组里每条指令对应一个枚举变体，加载期的兜底逻辑保证脚本漏写 `End` 也不会卡死。
- **已实现**：背景 / 立绘 / 对话框 / 选项分支 / flag 与变量 / 快速存读档 + 6 个存档槽 / 标题画面 / 系统菜单 / 设置页 / 历史回看 / 自动播放 / 中文字体。
- **尚未实现**：音频、转场特效、立绘补间、打字机 —— 这几条目前只解析不执行。

运行方式见上文 Build and run：把脚本放到 `scripts/test_chapter1.json`，然后 `cargo run -p celest_player`。
