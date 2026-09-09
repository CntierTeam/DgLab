---
name: dglab
description: >-
  Operate DgLab product CLI `dglab` by running it for the user: Coyote V2/V3
  scan, connect, setStrength, sendWave, battery, emergency zero. Prefer shell
  `--headless` execution over pasting recipes. Trigger on: DgLab, dglab, Coyote,
  郊狼, setStrength, sendWave, 强度, 波形, --mock, --headless.
license: MIT
metadata:
  short-description: 代跑 dglab（强度/波形/急停）
---

# DgLab

产品：**`dglab`** — 郊狼 Coyote **V2 / V3** 终端控制（**仅学习用**）。

你是 **操作员**：用户要扫描、连接、调强度、发波形、读电量、急停 → **自己在 shell 执行 `dglab`**，不要只拼命令给用户。

Agent 代控优先用 **`--headless`**（可管道喂命令）。TUI 需要交互终端，用户明确要全屏 UI 时再启动 `dglab` / `dglab --mock`。

## Agent 硬规则

1. **执行优先（全部功能）**：能跑就跑。二进制：`dglab` 或 `~/.local/bin/dglab`；没有就先装：
   `curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash`
   或仓库内 `./scripts/install.sh --from-source --force`。
2. **禁止**用「组装指令 / 操作手册 / SAMPLE / 长篇理科说明 / 我只能帮你校验」代替执行。短句说明 → 立刻跑 → 根据输出继续。
3. 用户问「能不能调强度 / 发波 / 连接」→ **先答能**，再 **马上执行** headless 流程。缺蓝牙地址时先 `scan`，再 `connect`；用户要真机就真机，不要默认改成 Mock 说教。
4. 命令名永远 **`dglab`**，禁止 `SAMPLE` / `YOUR_CLI`。
5. **所有** headless 动作都要会代跑：`scan` `connect` `getBattery` `setStrength` `getStrength` `sendWave` `setLimits` `setPulse` `zero`/`emergency` `disconnect` `stop`。
6. Mock（`--mock`）仅用户明确要自测或本机无蓝牙/无设备时；真机意图：`dglab --headless`（不加 `--mock`）。
7. **安全**：强度从低加起；脚本结束前务必 `zero`（或 `emergency`）再 `stop`；用户要停或异常时立刻急停，不要继续加压。
8. 不回显无关隐私；输出里关注 `msg` / `event`。

## 标准代跑流（headless）

```bash
command -v dglab || ~/.local/bin/dglab --help

# 无硬件 / 用户同意自测：
printf '%s\n' \
  scan \
  'connect AA:BB:CC:DD:EE:01' \
  'setStrength 1 0' \
  getBattery \
  zero \
  stop \
  | dglab --mock --headless

# 真机（把地址换成 scan 结果）：
printf '%s\n' \
  scan \
  'connect <MAC>' \
  'setStrength 1 0' \
  'sendWave A 5 135 20' \
  zero \
  stop \
  | dglab --headless
```

`--mock` 地址约定：`…:01` → V2，`…:03` → V3。

## 意图 → 怎么跑

| 用户意图 | 执行 |
|----------|------|
| 扫描设备 | headless 喂 `scan`，读 `event deviceFound` |
| 连接 | `connect <地址>` |
| 调强度 | `setStrength <A> <B>`（从低开始） |
| 发波形 | `sendWave <A\|B> <x> <y> <z>` |
| 读电量 / 读强度 | `getBattery` / `getStrength` |
| V3 软上限 / 脉冲 | `setLimits` / `setPulse` |
| 急停 | `zero` 或 `emergency` |
| 结束 | `zero` → `stop` |
| 要 TUI | 启动 `dglab` 或 `dglab --mock`（交互），并告知按键 |

## 启动模式

| 命令 | 用途 |
|------|------|
| `dglab --headless` | **Agent 代控首选**（真机 CLI） |
| `dglab --mock --headless` | 无硬件代跑 / 自测 |
| `dglab` | 用户要真机 TUI |
| `dglab --mock` | 用户要模拟 TUI |

## TUI 键位（用户自己玩时）

| 键 | 作用 |
|----|------|
| `s` / `c` / `g` | 扫描 / 连接 / 电量 |
| `[` `]` / `-` `=` | A / B 强度 |
| `w` `Enter` | 波形通道 / 发送 |
| `Space` / `0` | 紧急置零 |
| `q` | 退出 |

更全表：[references/cli-tui.md](references/cli-tui.md)。

## 安装（仅当本机没有 dglab）

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/DgLab/main/scripts/install.sh | bash
```

https://github.com/CntierTeam/DgLab
