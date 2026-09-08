# DgLab protocol & headless reference

Read this when changing BLE packing, headless commands, or event names.

## Device discovery

| Name prefix | Guess |
|-------------|-------|
| `D-LAB ESTIM` | V2 |
| `47L121` | V3 (Coyote 3) |

Connect version is confirmed by GATT service UUID (not name alone):

| Service UUID | Version |
|--------------|---------|
| `955a180a-0fe2-f5aa-a094-84b8d4f3e8ad` | V2 |
| `0000180c-0000-1000-8000-00805f9b34fb` | V3 |

## V2 characteristics

| Role | UUID |
|------|------|
| Battery | `955a1500-0fe2-f5aa-a094-84b8d4f3e8ad` |
| Strength | `955a1504-0fe2-f5aa-a094-84b8d4f3e8ad` |
| Wave B | `955a1505-0fe2-f5aa-a094-84b8d4f3e8ad` |
| Wave A | `955a1506-0fe2-f5aa-a094-84b8d4f3e8ad` |

### Strength (3 bytes LE)

Aligned with DGLAB-BT write path:

```text
ABits = (a * 7) & 0x7FF
BBits = ((b * 7) & 0x7FF) << 11
payload = (BBits | ABits) as u24 little-endian
```

Decode must unpack the same 11-bit fields (do **not** divide raw bytes by 7).

### Wave (3 bytes LE)

```text
packed = (x & 0x1F) | ((y & 0x3FF) << 5) | ((z & 0x1F) << 15)
```

Channel A → `955a1506`, channel B → `955a1505`.

## V3 characteristics

| Role | UUID |
|------|------|
| Write | `0000150a-0000-1000-8000-00805f9b34fb` |
| Notify | `0000150b-0000-1000-8000-00805f9b34fb` |
| Battery | `00001500-0000-1000-8000-00805f9b34fb` |

### B0 (20 bytes, every 100 ms)

- `0xB0` + seq/mode nibble + strength A/B + 4× freq/int for A + 4× freq/int for B
- Intensity modes: none / relative± / absolute (see `protocol/v3.rs`)
- Notify `0xB1` returns strength feedback
- `0xBF` updates soft limits / balances

Keep dungeonctl B0 hex vectors as regression tests only — do not depend on that crate.

## Headless commands

Compatible with DGLAB-BT unless marked **ext**.

| Input | Meaning |
|-------|---------|
| `scan` | Start scan |
| `connect <addr>` | Connect |
| `getBattery` | Read battery |
| `setStrength <A> <B>` | Set strength |
| `getStrength` | Read strength |
| `sendWave <A\|B> <x> <y> <z>` | Send wave (V2 write; V3 maps into pulse pattern) |
| `stop` | Quit |
| `emergency` / `zero` | **ext** emergency zero |
| `setLimits <A> <B>` | **ext** V3 BF limits |
| `setPulse <freqHz> <intensity>` | **ext** V3 pulse pattern |
| `disconnect` | **ext** disconnect keep process |

## Headless events

Stdout lines:

- `msg <text>`
- `event <name> [args...]`

Names: `start`, `stop`, `scanStart`, `scanComplete`, `deviceFound`, `deviceVersion`, `connectStart`, `connectSucceed`, `connectFailed`, `updateBattery`, `updateStrength`, `updateWave`.

## TUI keys (summary)

`q` quit · `s` scan · `c` connect · `g` battery · `Space`/`0` e-stop · `[`/`]` A ± · `-`/`=` B ± · `w` wave channel · `Enter` send wave.
