# 欢迎您对该项目作出贡献！

**写在前面**：

感谢您对 JustEnoughVCS 的关注和支持！如果您希望为项目做出贡献，请先仔细阅读本指南。

我们非常欢迎并重视每一位贡献 者的提交，但为了确保贡献流程的顺畅高效，请按照规范的方式进行提交。



## 第一步：选择您需要贡献的模块

`JustEnoughVCS` 采用模块化架构，将核心功能与客户端工具分离为不同的代码库：

| 仓库     | 链接                                                         | 描述                       |
| -------- | ------------------------------------------------------------ | -------------------------- |
| 核心逻辑 | [Version Control](https://github.com/JustEnoughVCS/VersionControl) | 主要运行逻辑、资产管理方案 |
| 前端     |                                                              |                            |
| 命令行   | [Command Line](https://github.com/JustEnoughVCS/CommandLine) | 核心到命令行的 “胶水代码”  |

请先了解您要为 `JustEnoughVCS` 的哪个模块提交代码，这很重要！

1. 如果您要优化 `JustEnoughVCS` 的版本控制逻辑、增加或修改核心功能，请前往 `VersionControl`。

2. 如果你要优化、修改、增加 命令行、图形界面 等前端调用的逻辑，请前往对应的库。



## 第二步：部署项目

请分叉你需要修改的库到您的 GitHub 账户，然后使用 SSH 克隆至本地。

对于非 `VersionControl` 部分，请将 核心 部分克隆至同级目录，以确保该部分可以引用到 `VersionControl` 仓库。

结构如下：

```
.
├── <前端名称>          # 前端 & 拓展
│   ├── src/
│   ├── Cargo.toml
│   └── README.md
└── VersionControl/    # 核心库
    ├── src/
    ├── Cargo.toml
    ├── CONTRIBUTING.md
    ├── LICENSE-MIT.md
    └── README.md
```



> [!NOTE]
>
> 目前不使用 `git submodule` 的原因是核心库和前端需要大量的同步修改
>
> 项目稳定后会转变为 `git submodule` 方式



## 第三步：部署开发环境

开发环境的配置请参考对应仓库中的文档。

`JustEnoughVCS` 在不同前端方向的技术选型不一样。

例如：`CommandLine` 部分采用 Rust + Clap 构成命令行程序；而图形界面部分采用 Avalonia + .NET。



## 第四步：提交您的 PR

在此之前，请确保：

1. 在您的分叉仓库中创建功能分支，并基于该仓库的 `dev` 或 `docs` 分支进行开发
2. 实现您的功能或修复
3. 在 COMMIT 信息中描述您的更改，并推送至您的分叉
4. 创建 Pull Request 到主仓库的 `dev` 或 `docs` 分支



### 注意：我们不会接受的 PR

我们预期外的修改：

1. 没有在 Issues 栏目中讨论过的大型更改
2. 不降反增心智复杂度的功能

错误的合并分支：

1. 合并到 `dev` 分支的文档内容修改
2. 合并到 `docs` 分支的代码内容修改
3. 合并到 `main` 分支的修改



## 开源协议

`JustEnoughVCS` 不同项目的开源协议并不相同，例如当前的 `VersionControl` 使用的是 MIT License （详见 LICENSE-MIT.md 文件）；而 GUI部分 则使用的 `GPLv3` 的协议。



最后，感谢您对 `JustEnoughVCS` 的支持！
