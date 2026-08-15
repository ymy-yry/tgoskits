# R2--R4：协商、实际长度与取消

本轮把 UVC 流的三个事实边界保持一致：设备协商的 payload、控制器报告的每包实际长度、以及未完成请求的取消生命周期。

- `UvcDevice` 使用 PROBE control 中的 `dwMaxPayloadTransferSize` 选择最小充分的 High-Speed ISO IN alternate setting；无法满足或零 payload 时明确失败。
- `VideoStream::recv` 仅将每个 ISO 包的 `actual_length` 范围交给 `FrameParser`；包数量不匹配或长度越界会报错。
- `EndpointRequestFuture` 在 pending 状态被丢弃时取消同一个 `RequestId`；已完成请求不会重复取消。

## 证据范围

本 PR 的证据为 Rust 主机侧编译、单元测试和 lint。它不是 RK3588 摄像头出流、DMA 吞吐、控制回路时延或长期稳定性验证；这些仍需带设备的板端测试。
