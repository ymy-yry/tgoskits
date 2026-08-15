# USB Camera P0 回移至 starryos-merged 的准备清单

## 双仓权威记录

| 角色 | 仓库 | 基线 |
| --- | --- | --- |
| 实际实现仓库 | `D:\proj\_audit_tgoskits_rcore_dev` | `4d4310cf216da01c8a0c098d04042d6ea0343124`，分支 `codex/usb-camera-p0` |
| 回移目标 | `D:\proj\starryos-merged` | R0 文档提交 `b48c796` |

## 回移前置条件

1. 目标仓库以固定 SHA 导入完整 TGOSKits USB/StarryOS 依赖闭包，至少包括 `crab-usb`、`usb-if`、`crab-uvc`、`ax-driver` 与 StarryOS `usbfs`。
2. 导入在 Linux/WSL 文件系统中完成；Windows checkout 会因 tracked path `drivers/sdmmc/src/emmc/aux.rs` 受保留名限制而不可靠。
3. 先验证导入 SHA、Cargo workspace members 与 `ax-driver` 的 `usb` feature，再 cherry-pick 本 P0 提交序列。
4. 每个回移提交必须重新执行目标工作区的 formatter、clippy、定向测试和 host 模拟；不得把 TGOSKits 的 host 结果移植为目标结果。

## 提交映射（持续更新）

| P0 轮次 | TGOSKits 提交 | 回移目标文件/前置 | 回移状态 |
| --- | --- | --- | --- |
| R0 | 待首次提交 | 本设计与本清单 | 未开始 |
| R1 | 待实现 | `drivers/usb/usb-host/.../xhci/device.rs` | 等待主链导入 |
| R2--R16 | 待实现 | UVC、usbfs、ax-driver 和测试 fixtures | 等待主链导入 |
