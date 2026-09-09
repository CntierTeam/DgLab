---
name: dglab
description: >-
  Operate the DgLab Coyote V2/V3 controller (`dglab`): install, TUI, and
  `--headless` line protocol compatible with DGLAB-BT. Use when the user wants
  to scan/connect a Coyote, set strength, send waves, emergency-stop, run mock
  demos, or script stdin/stdout control. Trigger on: DgLab, dglab, Coyote, 郊狼,
  DGLAB-BT, setStrength, sendWave, --mock, --headless. Not for rewriting the
  Rust codebase unless the user explicitly asks to develop it.
license: MIT
metadata:
  short-description: Use DgLab Coyote CLI/TUI
---

# DgLab（使用）

控制 DG-LAB 郊狼 **Coyote V2 / V3** 的命令行工具。二进制名：`dglab`。

**仅学习用** — 无担保；风险自负；遵守当地法律与设备安全；强度从低开始；随时可紧急置零。

## 何时用这个 skill

帮用户：**安装 / 启动 / 扫描连接 / 调强度发波形 / 写 headless 脚本 / 解释输出事件**。

不要默认去改仓库源码。只有用户明确说要开发、修 bug、改协议实现时，才读代码。

## 安装

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash
```

```powershell
# Windows PowerShell
iwr -useb https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Force
```

默认：

- 二进制 → `~/.local/bin/dglab`（Windows：`dglab.exe`）
- 本 skill → `~/.codex/skills/dglab`

确保 `~/.local/bin` 在 `PATH` 中。仓库与 Release：https://github.com/CntierTeam/DgLab

## 启动方式

| 命令 | 含义 |
|------|------|
| `dglab` | 真机 BLE + **TUI**（需要交互终端） |
| `dglab --mock` | 模拟设备 + TUI（无需蓝牙） |
| `dglab --headless` | 真机 + **行协议**（stdin/stdout） |
| `dglab --mock --headless` | 模拟 + 行协议（脚本/冒烟） |

无 TTY 时不要开 TUI，程序会提示改用 `--headless`。

Linux 真机需 BlueZ；Windows 走系统蓝牙；`--mock` 不需要适配器。

## 推荐操作流

1. 有硬件：`dglab` → `s` 扫描 → 选设备 → `c` 连接 → 小步调强度 → 需要时 `Space`/`0` 急停。
2. 无硬件练习：`dglab --mock`，或 headless：

```bash
dglab --mock --headless <<'EOF'
scan
connect AA:BB:CC:DD:EE:01
getBattery
setStrength 5 3
sendWave A 5 135 20
zero
stop
EOF
```

Mock 地址约定：`…:01` → V2，`…:03` → V3。

## TUI 按键

| 键 | 作用 |
|----|------|
| `q` | 退出 |
| `Tab` | 切换焦点 |
| `s` | 扫描 |
| `c` | 连接选中设备 |
| `g` | 读电量 |
| `Space` / `0` | **紧急停止（置零）** |
| `↑`/`↓` | 设备列表 |
| `[` / `]` | A 强度 −/+ |
| `-` / `=` | B 强度 −/+ |
| `w` | 波形通道 A/B |
| `Enter` | 发送波形 |

## Headless 行协议

一行一条命令；输出为 `msg ...` 或 `event ...`（对齐 DGLAB-BT）。

常用输入：

| 命令 | 说明 |
|------|------|
| `scan` | 扫描 |
| `connect <地址>` | 连接 |
| `getBattery` | 电量 |
| `setStrength <A> <B>` | 双通道强度 |
| `getStrength` | 读强度 |
| `sendWave <A\|B> <x> <y> <z>` | 发波形 |
| `emergency` / `zero` | 紧急置零 |
| `setLimits <A> <B>` | V3 软上限 |
| `setPulse <freqHz> <intensity>` | V3 脉冲 |
| `disconnect` | 断开（进程可继续） |
| `stop` | 退出进程 |

完整命令/事件表：见 [references/protocol.md](references/protocol.md)。

## 安全习惯（必须遵守）

1. 先 `zero` / 急停键，再调参；强度从低到高。
2. 脚本结束前务必 `zero`（或 `emergency`）再 `stop`。
3. 用户喊停或出现异常 → 立刻置零，不要继续加压。
4. 不要替用户绕过硬件上限或建议危险参数。

## 代理应答要点

- 直接给可复制的命令；优先 `--mock` 演示，真机前先确认用户有设备与蓝牙权限。
- 解释 `event`/`msg` 时用用户语言（用户用中文就回中文）。
- 区分 V2（名称常含 `D-LAB ESTIM`）与 V3（常含 `47L121`）。
