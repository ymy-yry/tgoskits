# StarryOS RK3588 USB Camera P0 设计与验证契约

## 问题、受众与完成标准

RK3588 上的 UVC 摄像头使用 HS/SS isochronous endpoint 时，xHCI endpoint context 的 interval 字段必须按 xHCI 语义编码。当前 `crab-usb` 将 USB descriptor 的 `bInterval` 直接写入该字段，形成一档偏移；这会使摄像头流传输调度不符合预期。

直接受众是 `crab-usb` 的 xHCI 用户、StarryOS `usbfs` 和 UVC 流驱动。R1 完成的可观察标准是：HS/SS ISO 的 `bInterval=1/2/16/17` 分别映射为 `0/1/15/15`，FS/LS 和非 ISO 行为不变，并由无硬件单元测试覆盖。R1 不完成 UVC 的协商、cancel/drain、RelationJoint、usbfs 生命周期或实板验收；这些分别属于 R2--R16。

## 资料与方案选择

| 方案 | 结论 |
| --- | --- |
| 保持现状 | 拒绝：不能满足 xHCI interval 编码语义。 |
| 在 StarryOS 调用方修正 | 拒绝：endpoint context 是 `crab-usb` xHCI 驱动职责，调用方没有正确的硬件边界。 |
| 在 xHCI 建链处修正并添加纯函数测试 | 采用：状态唯一、依赖方向不变、可在 host 侧稳定回归。 |
| 新建摄像头专用调度模型 | 拒绝：会和通用 xHCI 端点状态形成双重事实。 |

外部语义基准为 xHCI Specification §6.2.3.6：HS/SS periodic endpoint 的 interval 使用 `bInterval - 1` 编码。实现仅在 `drivers/usb/usb-host/src/backend/kmod/xhci/device.rs` 的 endpoint-context 建链路径生效。

## 所有权、并发与错误边界

本轮不改变 DMA、IRQ、锁、取消或端点所有权。映射函数只处理已验证的 descriptor 值，并返回受限 `u8`；`bInterval=0` 在输入不合规时饱和到零，避免 underflow。实际 endpoint context 仍由现有 `XhciDevice` 生命周期拥有。

## 验证映射

| 风险 | 验证 | 通过条件 |
| --- | --- | --- |
| HS/SS ISO off-by-one | `cargo test -p crab-usb xhci_interval` | 1→0、2→1、16→15、17→15 |
| 非目标回归 | 相同单测的 FS/LS 与 interrupt 分支 | 原有语义不变 |
| 代码质量 | `cargo fmt --all --check`、目标 crate clippy | 均退出 0 |

## 回移准备

实现提交将保持为单一、可 cherry-pick 的 `crab-usb` 改动。回移到 `starryos-merged` 前，必须先导入与本提交一致的 TGOSKits USB 主链及其 Cargo 依赖闭包；不能仅复制此文件。回移清单见 [BACKPORT.md](BACKPORT.md)。

