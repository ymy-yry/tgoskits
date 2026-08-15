# R2：UVC 协商 payload 与最小充分 alt setting

状态：`USB_CAMERA_UVC_NEGOTIATION_IMPLEMENTED`。

## 实现边界

`UvcDevice` 在 Probe/Commit 成功后保存设备返回的 `StreamControl`，启动流时仅使用其 `dwMaxPayloadTransferSize` 选择 ISO IN alternate setting。选择器依据完整 `wMaxPacketSize` 解码每 microframe 的多事务 payload，选择最小满足协商 payload 的 alt；零 payload、没有 ISO IN endpoint 或所有候选不足时显式返回错误。

本轮不把 `VideoFormat::frame_bytes()` 作为 USB payload 协商事实，也不偏好最大端点。取消、完成码、usbfs 生命周期和帧组装仍属于后续轮次。

## 已运行证据

```text
CARGO_NET_OFFLINE=true cargo test -p crab-uvc --lib --no-default-features
```

结果：5/5 通过。其中本轮覆盖：

- 最小充分 alt 选择；
- 零/不足协商 payload 的拒绝；
- HS high-bandwidth `wMaxPacketSize` 的事务数解码。

```text
CARGO_NET_OFFLINE=true cargo check -p crab-uvc --lib --no-default-features
CARGO_NET_OFFLINE=true cargo clippy -p crab-uvc --lib --no-default-features -- -D warnings
```

两项均通过。该证据是 host-side library 验证，不是 USB 实机、QEMU USB 或 RK3588 板端结果。
