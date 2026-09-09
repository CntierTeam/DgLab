# DgLab CLI / TUI 速查

仅学习用。只列产品操作，不含开发说明。

## 启动

| 命令 | 界面 |
|------|------|
| `dglab` | TUI + 真机 |
| `dglab --mock` | TUI + 模拟 |
| `dglab --headless` | CLI + 真机 |
| `dglab --mock --headless` | CLI + 模拟 |

## TUI 键位

| 键 | 作用 |
|----|------|
| `s` | 扫描 |
| `c` | 连接选中项 |
| `g` | 电量 |
| `↑` `↓` | 选设备 |
| `[` `]` | A 强度 −/+ |
| `-` `=` | B 强度 −/+ |
| `w` | 波形通道 A/B |
| `h` `l` / `j` `k` / `u` `i` | 调波形 x / y / z |
| `Enter` | 发送波形 |
| `Space` `0` | 紧急置零 |
| `Tab` | 焦点 |
| `q` | 退出 |

## CLI 命令（`--headless`）

| 输入 | 说明 |
|------|------|
| `scan` | 扫描 |
| `connect <addr>` | 连接 |
| `getBattery` | 电量 |
| `setStrength <A> <B>` | 设强度 |
| `getStrength` | 读强度 |
| `sendWave <A\|B> <x> <y> <z>` | 发波形 |
| `zero` / `emergency` | 置零 |
| `setLimits <A> <B>` | V3 上限 |
| `setPulse <freqHz> <intensity>` | V3 脉冲 |
| `disconnect` | 断开 |
| `stop` | 退出 |

## CLI 输出

| 行格式 | 含义 |
|--------|------|
| `msg <文本>` | 提示 |
| `event start` / `event stop` | 起停 |
| `event scanStart` / `scanComplete` | 扫描 |
| `event deviceFound <addr>` | 发现设备 |
| `event deviceVersion 2\|3` | 版本 |
| `event connectStart` / `connectSucceed` / `connectFailed` | 连接 |
| `event updateBattery <n>` | 电量 |
| `event updateStrength <A> <B>` | 强度 |
| `event updateWave …` | 波形 |

## 最小练习脚本

```bash
dglab --mock --headless <<'EOF'
scan
connect AA:BB:CC:DD:EE:01
getBattery
setStrength 1 0
zero
stop
EOF
```
