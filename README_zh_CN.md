<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/images/Header_Large.png" width="100%">
    </a>
</p>

<h1 align="center">JustEnoughVCS</h1>

<p align="center">
    让版本控制比呼吸还简单！
</p>


<p align="center">
    <img src="https://img.shields.io/github/stars/JustEnoughVCS/VersionControl?style=for-the-badge">
    <img src="https://img.shields.io/badge/Status-In%20Progress-yellow?style=for-the-badge">
    <img src="https://img.shields.io/badge/Release-Not%20Available-lightgrey?style=for-the-badge">
</p>


## 写在开头

如果您正在寻找一款让您团队成员可以轻松上手的版本控制，那么我们非常推荐您尝试一下。这款版本控制正如其名字讲述的那样：“刚好够用”，不产生过多的心智负担。



## 架构

`JustEnoughVCS` 是一款 **集中式** 的版本控制，分为 **客户端** 和 **服务端** 两个部分。

> [!NOTE]
>
> 该版本控制解决的是二进制资产的结构和版本管理的问题，不适合管理代码和文本。
>
> 在代码和文本领域，有极为成熟且被认可的其他 SCM 系统。



## 多文件结构

它鼓励用户以习惯的方式去放置、管理自己的资产，并按需将文件的 **可见性** 分享给团队内的其他成员。正因每位成员都有自己的结构，也不必担心文件的移动会影响他人。



## 串行编辑

同时，`JustEnoughVCS` 以文件为粒度进行权限管理，在同一时刻内，只有一名成员持有该文件，即拥有文件的 **编辑权**。在更新文件版本后，其他成员可在下次状态检查时发现新版本，并决定是否将最新版本更新到本地。



## 拒绝模糊映射

在实际文件结构与记录的文件结构有 **偏差** 时，`JustEnoughVCS` 将会禁止您追踪文件的版本，您需要解释清楚您本地的结构变动才能继续。



## 路线图

### 核心库

- [ ] 增量的文件更新和存储
- [ ] 多参照表



### 拓展工具

- [x] 命令行客户端 -> [CommandLine](https://github.com/JustEnoughVCS/CommandLine )
- [ ] 桌面客户端 -> [JVDesktop](https://github.com/JustEnoughVCS/JVDesktop )
- [ ] 声明式资产管理 -> [JVRefs](https://github.com/JustEnoughVCS/AssetsConfig) (目前 Private)
- [ ] 文件合并器 -> [JVBinMerger](https://github.com/JustEnoughVCS/BinMerger) (目前 Private)



## 支持

在使用 JustEnoughVCS 时遇到任何问题或有建议？

-   请将其提交到 https://github.com/JustEnoughVCS/VersionControl/issues 页面，我们将及时处理您的反馈。



## 许可证

本项目采用 **MIT 许可证**。

有关完整的许可证条款，请参阅项目根目录中的 ./LICENSE-MIT.md 文件。

---

最后，感谢您对 `JustEnoughVCS` 的关注！
