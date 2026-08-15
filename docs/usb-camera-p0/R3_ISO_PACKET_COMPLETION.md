# R3：等时包完成码保真链路

每个 ISO 包的真实长度与控制器 completion code 是不同的事实，不能以一次 transfer 的总体成功互相替代。

本轮扩展 `IsoPacketResult`：xHCI 后端将原始 8-bit TRB completion code（若可用）与每个包的 `actual_length` 一同传递到通用 USB 接口；其他后端保留 `None`。可移植的 `TransferStatus` 继续存在，但不再是唯一的信息来源。

本 PR 未实现 usbfs 的 per-packet Linux errno 映射，也未宣称板端视频流验证。后续映射应以该 completion-code 字段为输入，并配套独立测试。
