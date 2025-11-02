<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/images/Header_Large.png" width="100%">
    </a>
</p>

<h1 align="center">JustEnoughVCS</h1>

<p align="center">
    轻量且二进制友好的集中式版本控制系统
</p>

<p align="center">
    <img src="https://img.shields.io/github/stars/JustEnoughVCS/VersionControl?style=for-the-badge">
    <img src="https://img.shields.io/badge/Status-Development%20in%20Progress-yellow?style=for-the-badge">
    <img src="https://img.shields.io/badge/Release-Not%20Available-lightgrey?style=for-the-badge">
</p>

> [!WARNING]
> JustEnoughVCS 核心功能仍在开发，还不是可使用的状态
>
> 若您对我们的项目感兴趣，推荐直接联系我们。[联系方式和支持](#支持)



## 简介

一个文件同时只允许一人修改，这是二进制协作中最不容易出错的范式。JustEnoughVCS 将这种范式通过架构设计简化到极致。你可以随时掌握所关注文件的编辑状态、版本历史等信息。

### 1. 个人视图

每位成员拥有自己的表[^sheet]，通过表来映射资产内容与目录结构。让每位成员看到的文件结构各不相同，成员只需关注自己工作区中的资产位置，无需担心资产被他人移动，也不会因自己的移动操作影响他人。

### 2. 资产的 “读” 和 “写”

JustEnoughVCS 的权限模型非常直观：如果你看得见该资产，你就可以读取它；如果你持有该资产，则可以修改它。

[^sheet]: 表（Sheet），成员个人的文件结构，一位成员可以持有多个表，但一个本地工作区[^local_workspace]仅允许同时编辑一个。
[^local_workspace]: 本地工作区（Local Workspace），资产的本地拷贝目录，用以在成员的本地编辑文件。

### 3. 可读性传播

JustEnoughVCS 中，若您要将资产 **共享给所有人**，需要将该资产的映射导出[^export] 至参照表[^ref_sheet]，由库[^vault]管理员确认后将其公开展示给所有成员。之后所有成员都可以从参照表[^ref_sheet]中导入[^import]该资产的映射，以获取最新的更新。

若您只是需要共享给 **指定的表**，和上述逻辑一致，将资产可见性导出至该表，由该表的持有者确认后，则可以接收到您的文件更新。

[^export]: 导出（Export）是 JVCS 中资产映射的传递方式。
[^import]: 导入（Import）是 JVCS 中获得资产映射的方式。
[^ref_sheet]: 参照表（Reference Sheet）是由库[^vault]管理员所整理的表，团队中的 ”资产索引“
[^vault]: 库（Vault）是 JVCS 中的资产仓库，所有的资产存放于此。

### 4. 编辑权转移

一般来讲，资产的最初持有者是第一个追踪该资产的成员，该成员拥有绝对的资产编辑权。若该成员需要将编辑权转移给其他人，只需 ”放弃“ 编辑权，再由其他成员 ”持有“ ，此过程中会同步资产的最新进度，以确保持有该资产的成员正编辑的资产是最新的。



> [!NOTE]
>
> 该协作范式解决的是二进制资产的结构和版本管理的问题，不适合作为程序开发的版本控制系统。
>
> 因为该领域有极为成熟且被认可的 Git。



## 路线图

### 核心库

- [ ] 增量的文件更新和存储
- [ ] 多参照表



### 拓展工具

- [ ] 命令行工具 -> [CommandLine](https://github.com/JustEnoughVCS/CommandLine )（目前 Private）
- [ ] 声明式资产管理 -> [AssetsConfig](https://github.com/JustEnoughVCS/AssetsConfig) (目前 Private)
- [ ] 文件合并器 -> [BinMerger](https://github.com/JustEnoughVCS/BinMerger) (目前 Private)



## 支持

在使用 JustEnoughVCS 时遇到任何问题或有建议？

-   请将其提交到 https://github.com/JustEnoughVCS/VersionControl/issues 页面，我们将及时处理您的反馈。

> [!NOTE]
>
> 当前项目仍在早期开发阶段，比起发起 Issues，我更建议您直接联系仓库维护者 [@Weicao-CatilGrass](https://github.com/Weicao-CatilGrass)
>
> 待基础功能完善后，再发起 Issues 会更加合适



## 许可证

本项目采用 **MIT 许可证**。

有关完整的许可证条款，请参阅项目根目录中的 ./LICENSE-MIT.md 文件。

---

最后，感谢您对 `JustEnoughVCS` 的关注！
