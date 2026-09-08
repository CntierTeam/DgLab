---
name: dglab
description: >-
  Develop and operate the DgLab Rust Coyote V2/V3 BLE controller (binary `dglab`:
  ratatui TUI by default, `--headless` DGLAB-BT line protocol, `--mock` without hardware).
  Covers protocol encode/decode, DeviceSession, V3 100ms B0 tick, TUI/headless wiring,
  and verification. Trigger on: DgLab, dglab, Coyote, 郊狼, DGLAB-BT, btleplug Coyote,
  sendWave, setStrength, B0/B1, mock headless.
license: MIT
metadata:
  short-description: Coyote V2/V3 BLE CLI/TUI guide
---

# DgLab

Rust BLE controller for DG-LAB Coyote **V2 + V3**. Repo root is the Cargo package `dglab`.

**仅学习用** — educational / research only; no warranty; users accept their own risk and must follow local law and device safety.

## Hard rules

1. **Default argv → TUI**; **`--headless` → stdin/stdout line protocol**; **`--mock` → no adapter**.
2. Protocol bytes live only in `src/protocol/{v2,v3}.rs`. Change packing → update hex unit tests in the same files.
3. BLE I/O only through `DeviceSession` (`src/ble/`). UI/headless must not call `btleplug` directly.
4. App truth is `AppState` + `Command`/`Event` in `src/app.rs`. Event names stay aligned with DGLAB-BT (`event updateBattery`, etc.).
5. V3 **must** keep a **100ms B0 write loop** while connected; V2 writes on demand.
6. Prefer boring code. Do not pull in `dungeonctl` or copy GPL DGLAB-BT sources.

## Layout

| Path | Role |
|------|------|
| `src/main.rs` | clap: `--headless` / `--mock`; TTY guard |
| `src/app.rs` | `AppState`, `Command`, `Event` |
| `src/core.rs` | command router + V3 tick orchestration |
| `src/protocol/v2.rs` | V2 UUIDs + strength/wave encode/decode |
| `src/protocol/v3.rs` | B0/B1/BF + freq compression |
| `src/ble/session.rs` | `btleplug` session |
| `src/ble/mock.rs` | in-memory devices for tests/demo |
| `src/tui/` | ratatui UI |
| `src/headless.rs` | DGLAB-BT-compatible line I/O |

## Common workflows

### Build / run

```bash
cd "$(git rev-parse --show-toplevel)"
cargo build --release
cargo run -- --mock              # TUI + mock
cargo run -- --mock --headless   # line protocol
cargo run --release              # real BLE (BlueZ on Linux)
```

### Verify (no hardware)

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -q -- --mock --headless <<'EOF'
scan
connect AA:BB:CC:DD:EE:01
setStrength 5 3
sendWave A 5 135 20
getBattery
zero
stop
EOF
```

Mock addresses: `…:01` → V2, `…:03` → V3 (see `src/ble/mock.rs`).

### Protocol / feature work

1. Edit encode/decode in `protocol/v2.rs` or `v3.rs`.
2. Lock behavior with hex tests (`hex-literal`).
3. Wire `Command`/`Event` in `app.rs` + `core.rs` if the surface changes.
4. Update TUI and/or `headless.rs` together so both stay in sync.
5. Re-run `cargo test` and mock headless smoke.

V2 strength/wave packing and headless command table: read [references/protocol.md](references/protocol.md).

### Safety

Always keep an **emergency zero** path (`Command::EmergencyZero` / keys Space/`0` / `emergency`/`zero`). Prefer absolute strength 0 on V3 B0 when stopping.

## Install this skill into Codex

From GitHub Releases (preferred for users):

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash
# skill only:
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash -s -- --skill-only --force
```

From a local checkout (dev):

```bash
./scripts/install-codex-skill.sh --force          # symlink
./scripts/install.sh --from-source --force        # binary + skill
```

Destination: `${CODEX_HOME:-$HOME/.codex}/skills/dglab`.

## Out of scope (do not expand unless asked)

- Full PawPrints / other 47L12x app-layer opcodes
- Waveform IDE, cloud control, official App parity
