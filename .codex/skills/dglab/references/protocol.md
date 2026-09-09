# DgLab 使用参考：设备、命令与事件

面向**操作** `dglab`，不是改源码。仅学习用。

## 设备识别（扫描时）

| 名称特征 | 通常版本 |
|----------|----------|
| `D-LAB ESTIM…` | V2 |
| `47L121…` | V3 |

连接后以设备报告的版本为准（`event deviceVersion 2|3`）。

Mock（`--mock`）：

| 地址 | 版本 |
|------|------|
| `AA:BB:CC:DD:EE:01` | V2 |
| `AA:BB:CC:DD:EE:03` | V3 |

## Headless 输入

一行一条；字段空格分隔。

| 命令 | 说明 |
|------|------|
| `scan` | 开始扫描 |
| `connect <addr>` | 连接蓝牙地址 |
| `getBattery` | 读电量 |
| `setStrength <A> <B>` | 设 A/B 通道强度 |
| `getStrength` | 读当前强度 |
| `sendWave <A\|B> <x> <y> <z>` | 发波形参数 |
| `stop` | 退出程序 |
| `emergency` / `zero` | 紧急置零 |
| `setLimits <A> <B>` | V3：强度软上限 |
| `setPulse <freqHz> <intensity>` | V3：脉冲图案 |
| `disconnect` | 断开连接，保持进程 |

与 [DGLAB-BT](https://github.com/MossCG/DGLAB-BT) 兼容的核心命令：`scan` / `connect` / `getBattery` / `setStrength` / `sendWave` / `stop`。

## Headless 输出

- `msg <文本>` — 可读提示
- `event <名称> [参数…]` — 机器可读事件

常见事件：

| 事件 | 含义 |
|------|------|
| `start` / `stop` | 进程起停 |
| `scanStart` / `scanComplete` | 扫描起止 |
| `deviceFound <addr>` | 发现设备 |
| `deviceVersion 2\|3` | 版本 |
| `connectStart` / `connectSucceed` / `connectFailed` | 连接过程 |
| `updateBattery <n>` | 电量 |
| `updateStrength <A> <B>` | 强度 |
| `updateWave …` | 波形相关更新 |

## 脚本模板

```bash
dglab --mock --headless <<'EOF'
scan
connect AA:BB:CC:DD:EE:03
getBattery
setStrength 1 0
sendWave A 5 135 20
zero
stop
EOF
```

真机把 `connect` 换成扫描得到的地址，并去掉 `--mock`。结束前保留 `zero`。

## TUI 急停

`Space` 或 `0` → 紧急置零。退出用 `q`。
