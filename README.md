# DgLab — Coyote V2/V3 BLE CLI/TUI

> **仅学习用。** 本项目仅供协议学习与个人研究，不提供任何保证；使用风险自负，请遵守当地法律与设备安全规范。

纯 Rust 实现的郊狼 **V2 / V3** 蓝牙控制端：默认 **ratatui TUI**，另提供与 [MossCG/DGLAB-BT](https://github.com/MossCG/DGLAB-BT) 兼容的 **`--headless`** 行协议。无硬件时可用 **`--mock`**。

仓库：[CntierTeam/DgLab](https://github.com/CntierTeam/DgLab)

本仓库同时提供 **Codex Skill**（`$dglab`），把架构约束、协议要点和验证流程注入 Codex。

## 一键安装（从 GitHub Release）

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash
```

默认安装：

- 二进制 → `~/.local/bin/dglab`
- Codex skill → `~/.codex/skills/dglab`

常用选项：

```bash
# 指定版本
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh \
  | bash -s -- --version v0.1.0 --force

# 只装二进制 / 只装 skill
bash scripts/install.sh --bin-only
bash scripts/install.sh --skill-only --force

# 开发机：从本地源码安装
./scripts/install.sh --from-source --symlink-skill --force

# 卸载
./scripts/install.sh --uninstall
```

装好后确保 `~/.local/bin` 在 `PATH` 里，然后：

```bash
dglab --mock              # TUI + 模拟设备
dglab --mock --headless   # 行协议
```

## 从源码构建

```bash
rustup default stable
cargo build --release
cargo run -- --mock
```

Linux 真机 BLE 需要 BlueZ（及构建时的 `libdbus-1-dev` / `libudev-dev`）。

## CI / Release

| Workflow | 触发 | 作用 |
|----------|------|------|
| [CI](.github/workflows/ci.yml) | `push`/`PR` → `main` | `cargo test`、clippy、mock headless 冒烟 |
| [Release](.github/workflows/release.yml) | 推送 tag `v*.*.*` | 多平台 release 构建并发布 GitHub Release |

Release 产物：

- `dglab-<target>.tar.gz` — 预编译二进制（linux x86_64/aarch64、macOS aarch64）
- `dglab-skill.tar.gz` — Codex skill
- `install.sh` — 安装脚本副本
- 对应 `.sha256`

打 tag 发版：

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Codex Skill（仓库内）

```text
.codex/skills/dglab/
├── SKILL.md
├── agents/openai.yaml
└── references/protocol.md
```

- Release 安装：`./scripts/install.sh --skill-only`
- 本地开发 symlink：`./scripts/install-codex-skill.sh --force`

## 功能

| 功能 | V2 | V3 |
|------|----|----|
| 扫描 `D-LAB ESTIM*` / `47L121*` | ✓ | ✓ |
| 连接并识别服务 UUID | `955a180a-...` | `0000180c-...` |
| 电量 | 读 `955a1500` | 读/订阅 `00001500` |
| 双通道强度 | 写 `955a1504`（11bit LE） | B0 绝对/相对 + B1 回读 |
| 波形 | 单次写 A/B 3 字节 | **每 100ms** 写 B0（4×25ms） |
| 紧急停止 | 强度 0 + 波形清零 | 绝对强度 0 的 B0 |

## Headless 行协议（兼容 DGLAB-BT）

**输入（stdin，一行一条）：**

| 命令 | 说明 |
|------|------|
| `scan` | 扫描 |
| `connect <addr>` | 连接 |
| `getBattery` | 读电量 |
| `setStrength <A> <B>` | 设强度 |
| `getStrength` | 读强度 |
| `sendWave <A\|B> <x> <y> <z>` | 发波形 |
| `stop` | 退出 |
| `emergency` / `zero` | 紧急置零（扩展） |
| `setLimits <A> <B>` | V3 软上限（扩展） |
| `setPulse <freqHz> <intensity>` | V3 脉冲图案（扩展） |
| `disconnect` | 断开但保持进程（扩展） |

**输出：** `msg <文本>` / `event <名称> [参数...]`

Mock 约定：`…:01` → V2，`…:03` → V3。

```bash
printf 'scan\nconnect AA:BB:CC:DD:EE:03\nsetStrength 10 0\ngetBattery\nemergency\nstop\n' \
  | dglab --mock --headless
```

## TUI 按键

| 键 | 作用 |
|----|------|
| `q` | 退出 |
| `Tab` | 切换焦点区 |
| `s` | 扫描 |
| `c` | 连接选中设备 |
| `g` | 读电量 |
| `Space` / `0` | 紧急停止 |
| `↑`/`↓` | 设备列表 |
| `[` / `]` | A 通道强度 −/+ |
| `-` / `=` | B 通道强度 −/+ |
| `w` | 切换波形通道 A/B |
| `Enter` | 发送波形 |

## 源码地图

| 路径 | 职责 |
|------|------|
| `src/main.rs` | 入口 / clap |
| `src/app.rs` | `AppState` + `Command`/`Event` |
| `src/core.rs` | 命令路由与 V3 tick |
| `src/protocol/` | V2/V3 编解码（含 hex 单测） |
| `src/ble/` | `DeviceSession`、真 BLE、Mock |
| `src/tui/` | ratatui |
| `src/headless.rs` | 行协议 |

## 协议归属

本仓库代码为 **MIT** 自研实现，**不复制** GPL 源码文件。

- V2 对齐 [DGLAB-BT](https://github.com/MossCG/DGLAB-BT) 写入路径；强度回读用正确的 11-bit unpack。
- V3 对齐 [DG-LAB-OPENSOURCE](https://github.com/DG-LAB-OPENSOURCE/DG-LAB-OPENSOURCE/blob/main/coyote/v3/README.md)；B0 测试向量对照 dungeonctl（不依赖该 crate）。

细节见 [.codex/skills/dglab/references/protocol.md](.codex/skills/dglab/references/protocol.md)。

## 测试

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --mock --headless <<'EOF'
scan
connect AA:BB:CC:DD:EE:01
setStrength 5 3
sendWave A 5 135 20
getBattery
zero
stop
EOF
```

## License

MIT — see [LICENSE](LICENSE).
