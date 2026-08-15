# USB 摄像头后续集成矩阵

| 范围 | 本 PR | 仍需的证据 |
| --- | --- | --- |
| UVC payload 与帧解析 | 使用每包实际长度 | RK3588 摄像头出流 |
| xHCI ISO 完成结果 | 保留每包原始 completion code | usbfs 映射与控制器故障注入 |
| 请求生命周期 | pending future Drop 时取消 | 断连/重连与长期压力测试 |
| DMA/性能 | 不改变 DMA 策略 | 板端吞吐、CPU、时延和 jitter 数据 |
| 舵机/轮盘 | 不在 tgoskits USB/UVC 接口范围内 | 以独立 PWM/控制器驱动 PR 提交 |

结论：该分支是 USB/UVC P0 的主机侧 PR，不应被表述为完整的 StarryOS 摄像头、DMA 性能或实体执行器验证。
