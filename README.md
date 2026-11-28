# chrono-harness-judge-ci

核对执行单元的原始结果并汇总 CI 状态。

在仓库根目录运行 `mise exec -- ./check.sh`。本仓的 mise 配置固定 Verus 版本，按变更验证CI 原件准入与汇总。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-ci)

产品和测试从本仓源码与 crates.io 锁文件独立构建。运行时通过宿主安装的 `chrono-harness` 获取固定两端的 Git 事实、原进程回执、受保留策略管理的文件和准备阶段身份，通过 `chrono-judge-routes` 查询执行单元、计划与回执。部署时使用本仓 mise 配置中的发行版本及 SHA-256。

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

scoped CI 遇到 schema 2 的项目登记时，通过 `chrono-judge-projects check-pairs-v1` 核对变更涉及的项目或脚本。宿主在工具登记中声明 `chrono-judge-projects` 时按其 `program` 启动，否则从 PATH 解析。调用使用当前配置登记的时间和输出上限。宿主从本仓 mise 配置所列的固定发行安装项目判官。

scoped 调度使用本仓 mise 配置中的独立 FILEMAP 发行。宿主可在工具登记的 `chrono-judge-filemap.program` 指定安装位置，默认从 PATH 选择。完整调度和收集从 PATH 调用 FILEMAP 和 routes，FILEMAP 通过 mixed 分类变更。调度沿用已登记的 Git 环境。收集使用原请求和登记的历史转换原件。响应中的 `git_facts` 与 `filemap_git_facts` 分别记录 CI 和 FILEMAP 实际发生的 Git 调用。

直接 scoped 配置的 `policy.environment` 提供操作使用的固定值，`policy.inherit_environment` 列出从调用者读取的变量名。工具版本和操作收据会发布这些选中输入。使用 `registration_config` 的宿主在登记配置的 `environment.inherit` 中选择继承项。

scoped CI 执行已选操作时使用宿主安装的 `chrono-judge-projects execute-plan-v1` 和 `chrono-judge-routes`。可在宿主的 tools 登记这两个程序的路径，未登记时按 PATH 查找。CI 从 projects 接收每个操作的回执和原始流，在本仓核对计划和操作集合，并把实时诊断写入宿主选择的日志。业务失败和诊断写入失败分别进入检查结果。

scoped 计划从已安装的 routes 查询。调度和回放使用 `scoped-operations-v1` 展开已选操作，准备执行时使用 `prepare-scoped-plan-v1` 观察登记的工具并生成计划。CI 核对返回的宿主根、绑定、所选测试与调度参数，再交给 projects 执行。本仓 mise 配置固定了支持这两个命令的 routes 发行和各平台摘要。

收集单元原件时，CI 展开已留存的进程输出，将整份计划和回执交给同一个已安装的 routes。它校验返回的计划身份和操作集合，按计划顺序处理每个回执的比较结果，并继续检查本仓负责的证明输出和单元结果。

完整 CI 调度与收集调用 routes 的 `full-operations-v1`。它们传入各端点已解释的登记和当前测试的操作绑定，包括历史解释结果。调度核对所有测试的单元归属，收集回放原始操作顺序。完整收集先校验登记路径和父目录，再用操作系统解析链接并比较规范路径。Verus 判法使用该路径比较结果和 JSON 解码得到的声明版本与摘要。只有声明一致才读取真实文件并比较 SHA-256，文件读取失败沿用原诊断。

完整与 scoped CI 都将两端的登记、受影响节点和有序操作提交给 routes 的 `source-plan-v1`。judge-ci 核对返回的提交与 Git 绑定，将 routes 的读取按实际发生顺序写入 `git_facts`，并将其判定错误交给调用方。

scoped CI 把原请求的绑定字段及报告路径交给 routes 生成计划，收集时用 `check-scoped-plan-v1` 校验原件。生产 Cargo 清单仅按 JSON 协议消费 routes。源码库存与所选根的组合验证由 runner 和 routes 分别维护。

scoped CI 用候选宿主的 chrono-judge-registration 执行 `scoped-execution-v1`，由登记判官在同一次调用中校验项目、执行计划限额、调度和引用语法，返回测试的终止操作和所有者。CI 使用该次校验对应的原始计划和调度声明。含 `configuration_edges` 的端点先将完整 FILEMAP 和有效配置交给同一已安装工具的 `configuration-edges-v1`，把选中的附加边纳入影响计算。基线和候选使用各自的登记，历史 v1 项目登记沿用其原有绑定规则。宿主可在工具登记中指定该命令的位置，省略时按 PATH 解析。退役与替换判断沿用两端各自的绑定。

使用 scoped CI 的宿主可以在检查策略中登记 `causal_consumption`。CI 判官在基线和候选提交中读取启动配置，核对所选生产、安装和使用命令，并为受变更影响的关系选择已登记的证据测试。它在本仓验证准备命令及前置条件。

启用 v4 登记的 scoped 宿主安装支持 `check-documents-v1` 的 chrono-judge-registration。CI 在首次检查和两个提交的比较中把固定候选文档交给已登记的安装程序，核对它返回的有效配置路径，再验入口绑定。

采用 `execution_diagnostics: "chrono-log-stderr/v1"` 的宿主需安装支持 `trace-events-v1` 的 chrono-diagnostics。可以在同一份已绑定的 policy.tools 中指定其路径，默认从 PATH 解析。显式日志流、宿主滚动文件和业务回执可同时使用。日志写入失败会作为独立检查结果保留，业务操作的退出码及原始回执仍可读取。

在 scoped 配置声明 `policy.registration_config` 的 DELTA 检查中，CI 从宿主选中的 `chrono-judge-registration` 调用 `ci-transition-v1`。该命令自登记工具 `v0.1.0-alpha.20` 提供。CI 核对提交身份与所选退役测试，保留历史转换和原成本，并把额外的 Git 进程按调用顺序写入现有报告。

完整调度和收集使用已安装的 registration 解释两端登记与测试操作。调度遇到需要解码器的历史配置时拒绝执行。收集只从原报告提供方请求 registration 实际选中的迁移单元。scoped 启动消费从同一次端点检查中取得方法、计划和测试所有者。调度声明变化时才查询受资源或输出争用影响的测试。以上路径自 registration `v0.1.0-alpha.21` 提供。

CI 的 Rust 产品及测试清单不再依赖 registration 源码。历史 Rust 再导出的快照、保留记录和引用语法函数由 registration 提供，原生用例和适用证明在该仓库执行。CI 接收公开命令的结果并保留自己的业务选择与回执。

完整调度从 PATH 中的 `chrono-harness git-facts-v1` 读取基线和候选登记。两次配置核验、两端快照和 Git 进程记录来自同一个已安装的 runner 会话。CI 使用候选快照指定的有效配置选择超时和输出限额。

## 单元汇总与准备条件的证明

在本仓运行 `mise exec -- ./check.sh`。该命令构建实际库、运行原生测试，对以下 Verus 根分别调用固定验证器，并以编译器生成的依赖清单、执行回执和原始输出逐项核对。

| 生产根和使用方 | 判定 | 独立预期 |
| --- | --- | --- |
| `verified_collection.rs`，由 `collect.rs` 和 `lib.rs` 调用 | 由单元结果归约汇总状态，核对所选操作、结果集合及执行状态 | `judge-ci-tests/tests/verified_collection.rs` |
| `verified_consumption.rs`，由 `consumption.rs` 调用 | 对照登记，核对准备步骤的命令和验证前置操作 | `judge-ci-tests/tests/verified_consumption.rs` |

两组测试根也由同一入口独立验证。GitHub 提供的 run、attempt、所选单元及下载归 `chrono-ci` 负责。`requirements-v1` 的 `unit-originals` 根据宿主配置、传输位置与摘要读取原报告及其准备收据。

## 本仓本地检查的清理

开发者运行 `mise exec -- ./check.sh` 时，工作树租约覆盖整个检查。准备输入和原报告写入 `.chrono-harness/state/preparation/`，单元结果在 `.chrono-harness/state/admission/`，汇总在 `.chrono-harness/state/collection/`。证明编译库和依赖清单进入 `admission-proofs/`，保留至原生检查读取完毕并交接回执，原生构建使用专属的 `native-check/` 临时目录。需要保留的结果应在当日清理前复制到持久存储。

在主检出运行 `mise exec -- chrono-worktree maintain --host-root . --config .chrono-harness/worktree.json
```` 可执行一轮工作树维护。独占执行者核对身份、租约与源码保全情况，然后回收符合条件的产物。

同一日历批次共用截止时刻。调度器在运行且取得排他租约时回收已登记的空闲目录。首次登记前已有的目录需要核对、转存并显式接管。普通任务工作树在超过 24 小时且唯一源码已保全后可整树回收；仍在使用的原件继续保管。工作树日志和处置报告在一天的窗口及当前使用方结束后滚动。工具包位于 mise 共享安装目录，本仓的 bin/ 只保存链接。

限定范围的调度和收集读取 routes v2 计划。超过 9007199254740991 的操作超时在计划、工具观察与回执中采用十进制字符串，重读时恢复完整 u64，校验由已安装的 routes 执行。
