<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/Resources/Visual/Yizi_Icon.png" width="20%">
    </a>
</p>
<h1 align="center">JustEnoughVCS</h1>

<p align="center">
    多文件结构的集中式版本控制
</p>


<p align="center">
    <img src="https://img.shields.io/github/stars/JustEnoughVCS/VersionControl?style=for-the-badge">
    <img src="https://img.shields.io/badge/Status-In%20Progress-yellow?style=for-the-badge">
    <img src="https://img.shields.io/badge/Release-Not%20Available-lightgrey?style=for-the-badge">
</p>

## 写在开头

> [!Warning]
>
> `JustEnoughVCS` 还在稳步开发，目前并无发布的计划
>
> 如果您对该项目感兴趣，欢迎直接[联系](#支持)我们！

> [!NOTE]
>
> `JustEnoughVCS` 为**资产内容管理**而设计，并**不适合作为源代码版本控制**，多结构、无分支的设计理念决定了其无法胜任任何强结构依赖的项目。

## 介绍

`JustEnoughVCS` 是一款 **集中式版本控制系统**，它专注于为 **以二进制文件为主的资产管理场景** 提供简洁、自由的工作流。它不将文件路径视为文件的身份，而是分离成 **唯一身份（文件是什么）** 和 **路径映射（文件在哪）** 的关系。

> [!TIP]
>
> 若您希望直接安装，可以点击[此处](#安装)

 

### 一、结构表系统 (Struct Sheet System)

**个人结构表**

`JustEnoughVCS` 允许每名成员都建立自己的 **结构表**，它是 **该成员对于自己可见文件的理解**，它不会也不需要被其他成员知道长什么样。每名成员都在自己的 **结构表** 内工作，在需要的时候将部分 **文件可见性** 分享给他人即可。

**参考结构表**

在 `JustEnoughVCS` 的上游仓库（Upstream Vault）中，会由 **仓库管理员** 维护一份公开可见的 **参考结构表** (Reference Sheet)，所有成员都可以查询到 **文件在参考结构表中的位置**，以方便建立对文件的理解。

<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/Resources/Visual/Image2.png" width="100%">
    </a>
    <p align="center">
    	参考结构表用于锚定文件的权威命名
	</p>
</p>



### 二、串行编辑

`JustEnoughVCS` 采用 **串行编辑模型**，从根本上消除合并冲突。我们的权限原则是：**可见即可读，持有则可写**。

当您持有文件时，您拥有独占的编辑权。其他成员可以看到当前版本，但无法同时编辑。编辑完成后，您提交的新版本立即可供所有人获取。

串行编辑确保只有一个“真相来源”。如果有成员在未持有权限或基于过时版本进行编辑，系统会明确标记这些修改为**无效编辑**，也就是说，它们不会被意外提交或覆盖有效版本。

不过不用担心，在 `JustEnoughVCS` 的前端（[命令行、桌面端](#安装)）中，提供了清晰可见视图，用于确认文件状态。



### 三、架构

`JustEnoughVCS` 为集中式版本控制，分为 **服务端** 和 **客户端**，本地工作区仅为用于记录修改和更新的文件拷贝。

**上游库 (Upstream Vault)**

上游库记录了如下信息：

1. 上游库配置信息
2. 成员注册信息及公钥
3. 结构表
   1. 当前持有者
   2. 映射关系
4. 文件元数据
   1. 当前持有者
   2. 版本顺序
5. 文件历史版本存储**（全量、未压缩）**



**工作区 (Local Workspace)**

工作区记录了如下信息：

1. 工作区配置信息
2. 上游库元数据的本地缓存
3. 结构表的本地缓存：用来对比本地和上游差异
4. 本地结构表：用来分析工作区变化
5. 物理文件

<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/Resources/Visual/Image1.png" width="100%">
    </a>
    <p align="center">
    	Vault 和 Workspace 的关系
	</p>
</p>



## 安装

该仓库为 `JustEnoughVCS` 的核心库，不提供任何交互功能。

请前往对应的仓库以下载或构建 `JustEnoughVCS` 的前端：

- [命令行前端](https://github.com/JustEnoughVCS/CommandLine)
- [桌面端](https://github.com/JustEnoughVCS/Desktop) **开发中**



## 路线图

- [ ] 核心功能 

  - [x] 表管理
  - [x] 追踪
  - [x] 对齐
  - [x] 分享
  - [x] 参考
  - [x] 管理员工具
  - [ ] 版本跳转
  - [ ] 版本冻结
  - [ ] 借用器

- [ ] 命令行

  - [ ] 完整核心功能封装

  - [x] 帮助
  - [x] 补全
  - [ ] 内嵌文档
  - [ ] JSON 输出

- [ ] 桌面端

  - [ ] 完整核心功能封装

  - [x] 命令行包装器
  - [ ] 主题

- [ ] 2026 计划

  - [ ] 桌面端支持 **当前阶段**

  - [ ] 规范化核心、命令行代码
  - [ ] 提高异步、并发支持
  - [ ] 存储优化
    - [ ] 分布式内容存储、集中式权限管理




## 支持

在使用 JustEnoughVCS 时遇到任何问题或有建议？

-   请将其提交到 https://github.com/JustEnoughVCS/VersionControl/issues 页面，我们将及时处理您的反馈。



## 许可证

本项目采用 **MIT 许可证**。

有关完整的许可证条款，请参阅项目根目录中的 *./LICENSE-MIT.md* 文件。

---

最后，感谢您对 `JustEnoughVCS` 的关注！



## 一些碎语

> 为什么要给它命名 `JustEnoughVCS` ？
>
> 如果你认为它听起来像是 Minecraft 中的 Mod 才会取的名字，那你就猜对了！
>
> 其实灵感正是来自 `JustEnoughItems` 😄
