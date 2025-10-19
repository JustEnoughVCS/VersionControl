# 欢迎您的贡献！

欢迎您对 `JustEnoughVCS` 进行贡献！开始之前，请阅读本指南以确保您的贡献流程顺利。



## 一、选择您的方向

`JustEnoughVCS` 采用模块化架构，将核心功能与客户端工具分离为不同的代码库：

| 仓库     | 链接                                                         | 描述                       |
| -------- | ------------------------------------------------------------ | -------------------------- |
| 核心逻辑 | [Version Control](https://github.com/JustEnoughVCS/VersionControl) | 主要运行逻辑、资产管理方案 |
| 前端     |                                                              |                            |
| 命令行   | [Command Line](https://github.com/JustEnoughVCS/CommandLine) | 核心到命令行的 “胶水代码”  |

请先了解您要为 `JustEnoughVCS` 的哪个模块提交代码，这很重要！

1. 如果您要优化 `JustEnoughVCS` 的版本控制逻辑、增加或修改核心功能，请前往 `VersionControl`。

2. 如果你要优化、修改、增加 命令行、图形界面 等前端调用的逻辑，请前往对应的库。



## 二、部署项目

请分叉你需要修改的库到您的 GitHub 账户，然后使用 SSH 克隆至本地。

对于非 `VersionControl` 部分，请将 核心 部分同时以 HTTP 的方式克隆至同级目录，以确保该部分可以引用到 `VersionControl` 仓库。

结构如下：

```
.
├── <前端名称>
│   ├── src/           # 前端源代码
│   ├── Cargo.toml     # Rust 项目配置
│   └── README.md      # 项目说明文档
└── VersionControl/    # 核心库引用
    ├── src/
    ├── Cargo.toml
    ├── CONTRIBUTING.md
    ├── LICENSE-MIT.md
    └── README.md
```



## 三、部署开发环境

开发环境的配置请参考对应仓库中的文档。

`JustEnoughVCS` 在不同前端方向的技术选型不一样。

例如：`CommandLine` 部分采用 Rust + Clap 构成命令行程序；而 `MyVault` 图形界面部分采用 Avalonia + .NET。



## 四、提交您的 PR

在此之前，请确保：

1. 在您的分叉仓库中创建功能分支
2. 实现您的功能或修复
3. 编写适当的测试用例，并通过测试
5. 提交清晰的提交信息
6. 创建 Pull Request 到主仓库

### 注意事项

- 请确保您的代码遵循项目的编码规范
- 在提交 PR 前，请同步主仓库的最新更改
- 对于重大更改，建议先在 Issues 中讨论设计方案



## 最后、开源协议

`JustEnoughVCS` 不同项目的开源协议并不相同，例如当前的 `VersionControl` 使用的就是最宽松的 MIT License （详见 LICENSE-MIT.md 文件）；而 `MyVault` 则使用的 `GPLv3` 的协议。



最后，感谢您对 `JustEnoughVCS` 的支持！
