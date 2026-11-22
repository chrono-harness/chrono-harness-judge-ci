# chrono-harness-judge-ci

核对执行单元的原始结果并汇总 CI 状态。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-ci)

## CI 输入查询

从被检查的 Git 宿主根目录运行 `chrono-judge-ci requirements-v1`。stdin 接收一个不超过 1 MiB 的 JSON 对象。成功时 stdout 返回一个 JSON 对象并退出 0，输入或判定失败时 stderr 给出原因并退出 2。所有路径均相对宿主。调用者可以单独安装本仓 release 中的平台二进制并用同一发行的 SHA256SUMS 校验。

| operation | 除 profile 之外的输入 | 成功结果 |
| --- | --- | --- |
| schedule | base、candidate、initial | required_units、global_selected、变更路径与 Git 观察 |
| collect | base、candidate、initial、manifest | required_units、global_selected、选择依据与 Git 观察 |
| full-schedule | base、candidate | full 配置下的调度要求与 Git 观察 |
| full-collect | base、candidate、context、scope | full 的汇总要求与 Git 观察。按需读取登记的历史转换报告 |
| unit-originals | profile_sha256、unit、report、transport | 通过原件核验后的 unit、report_sha256 |

其中 `report` 包含原发布路径 `path` 和 `sha256`。`transport` 的 `source_directory` 标识上传目录，`directory` 标识本次下载目录。原件读取沿用宿主的报告限制、摘要和准备记录校验。常规判官仍直接从 stdin 接收判定请求。
