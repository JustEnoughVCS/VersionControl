# JustEnoughVCS 贡献指南

欢迎您对 JustEnoughVCS（简称 `JVCS`）项目进行贡献！在开始贡献之前，请仔细阅读本指南以确保您的贡献流程顺利。

## 项目结构

JustEnoughVCS 采用模块化架构，将核心功能与客户端工具分离为不同的代码库：

### 核心仓库
- **核心库**: https://github.com/JustEnoughVCS/VersionControl
- **命令行前端**: https://github.com/JustEnoughVCS/CommandLine

## 流程

### 1. 准备工作

**对于核心库**：
- 分叉核心库到您的 GitHub 账户
- 使用 SSH 方式克隆到本地

**对于前端**：
- 分叉对应的前端仓库
- 使用 SSH 方式克隆到本地

### 2. 项目目录结构

所有前端项目都遵循统一的目录结构：

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

### 3. 开发环境设置

1. 确保已安装 Rust 工具链
2. 克隆项目到本地
3. 按照各仓库的 README 说明配置开发环境

### 4. 提交 Pull Request

1. 在您的分叉仓库中创建功能分支
2. 实现您的功能或修复
3. 编写适当的测试用例
4. 确保代码通过所有现有测试
5. 提交清晰的提交信息
6. 创建 Pull Request 到主仓库

## 注意事项

- 请确保您的代码遵循项目的编码规范
- 在提交 PR 前，请同步主仓库的最新更改
- 对于重大更改，建议先在 Issues 中讨论设计方案

感谢您对 JustEnoughVCS 项目的贡献！
