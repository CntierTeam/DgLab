---
name: dglab
description: >-
  Help the user operate the DgLab product CLI/TUI (`dglab`) for Coyote V2/V3:
  launch modes, TUI keys, headless commands, and emergency stop. Trigger on:
  DgLab, dglab, Coyote, 郊狼, setStrength, sendWave, --mock, --headless, TUI.
  Do not use this skill for source-code development.
license: MIT
metadata:
  short-description: DgLab CLI/TUI how-to
---

# DgLab

产品：`dglab` — 郊狼 Coyote **V2 / V3** 的终端控制端（**仅学习用**）。

本 skill **只说明怎么用 CLI/TUI**。不要改代码、不要讲仓库结构。

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash
```

Windows PowerShell：

```powershell
iwr -useb https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Force
```

装好后执行 `dglab`（Windows 为 `dglab.exe`）。需保证 `~/.local/bin` 在 PATH 中。

## 两种界面

| 启动 | 用途 |
|------|------|
| `dglab` | **TUI**（全屏按键操作，需交互终端） |
| `dglab --headless` | **CLI 行协议**（stdin 输命令，stdout 看结果） |
| `dglab --mock` | 无真机时的模拟设备（可与上面组合） |

示例：

```bash
dglab                  # 真机 TUI
dglab --mock           # 模拟 TUI
dglab --headless       # 真机 CLI
dglab --mock --headless
```

## TUI 怎么用

1. 启动后按 `s` 扫描。
2. `↑`/`↓` 选设备，`c` 连接。
3. `[`/`]` 调 A 强度，`-`/`=` 调 B 强度（从低开始）。
4. 需要发波形时调好参数后按 `Enter`。
5. **随时** `Space` 或 `0` 紧急置零；退出按 `q`。

| 键 | 作用 |
|----|------|
| `s` | 扫描 |
| `c` | 连接 |
| `g` | 读电量 |
| `[` `]` | A 强度 − / + |
| `-` `=` | B 强度 − / + |
| `w` | 切换波形通道 A/B |
| `Enter` | 发送波形 |
| `Space` / `0` | 紧急置零 |
| `Tab` | 切换焦点 |
| `q` | 退出 |

## CLI（`--headless`）怎么用

一行一条命令。常用：

```text
scan
connect <蓝牙地址>
getBattery
setStrength <A> <B>
getStrength
sendWave <A|B> <x> <y> <z>
zero
stop
```

扩展：`emergency`（同 zero）、`setLimits <A> <B>`、`setPulse <freq> <intensity>`、`disconnect`。

输出：

- `msg ...` 提示文字
- `event ...` 状态事件（如 `deviceFound`、`connectSucceed`、`updateBattery`、`updateStrength`）

管道示例：

```bash
printf 'scan\nconnect AA:BB:CC:DD:EE:01\nsetStrength 1 0\nzero\nstop\n' | dglab --mock --headless
```

`--mock` 时可用地址 `…:01`（V2）或 `…:03`（V3）练习。

更多命令表见 [references/cli-tui.md](references/cli-tui.md)。

## 安全（产品使用必守）

- 强度从最低加起；脚本结束前先 `zero` 再 `stop`。
- 用户要停或异常时，立刻急停，不要继续加压。
- 仅学习用，风险自负。
