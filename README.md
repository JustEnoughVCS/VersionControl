<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/images/Header_Large.png" width="100%">
    </a>
</p>

<h1 align="center">JustEnoughVCS</h1>

<p align="center">
    Making version control easier than breathing!
</p>


<p align="center">
    <img src="https://img.shields.io/github/stars/JustEnoughVCS/VersionControl?style=for-the-badge">
    <img src="https://img.shields.io/badge/Status-In%20Progress-yellow?style=for-the-badge">
    <img src="https://img.shields.io/badge/Release-Not%20Available-lightgrey?style=for-the-badge">
</p>


## Introduction

If you are looking for a version control system that your team members can easily get started with, we highly recommend you give this a try. This version control system lives up to its name: "Just Enough," providing just what you need without creating excessive mental overhead.



## Architecture

`JustEnoughVCS` is a **centralized** version control system, divided into two parts: the **client** and the **server**.

> [!NOTE]
>
> This version control system addresses the problem of managing the structure and versions of binary assets and is not suitable for managing code and text.
>
> In the realm of code and text, there are other extremely mature and widely recognized SCM systems.



## Multi-File Structure

It encourages users to place and manage their assets in their preferred way and share the **visibility** of files with other team members as needed. Since each member has their own structure, there's no need to worry about file moves affecting others.



## Serialized Editing

At the same time, `JustEnoughVCS` manages permissions at the file granularity. At any given moment, only one member holds a file, meaning they have the **editing rights** for that file. After updating a file version, other members can discover the new version during the next status check and decide whether to update the latest version to their local workspace.



## Rejecting Ambiguous Mappings

When there is a **discrepancy** between the actual file structure and the recorded file structure, `JustEnoughVCS` will prevent you from tracking the file's version. You need to clearly explain your local structural changes before you can proceed.


## Roadmap

### Core Library

- [ ] Incremental file updates and storage
- [ ] Multiple reference tables



### Extension Tools

- [x] Command-line client -> [CommandLine](https://github.com/JustEnoughVCS/CommandLine )
- [ ] Desktop client -> [JVDesktop](https://github.com/JustEnoughVCS/JVDesktop )
- [ ] Declarative asset management -> [JVRefs](https://github.com/JustEnoughVCS/AssetsConfig) (Currently Private)
- [ ] File merger -> [JVBinMerger](https://github.com/JustEnoughVCS/BinMerger) (Currently Private)



## Support

Encountering any issues or have suggestions while using JustEnoughVCS?

-   Please submit them to the https://github.com/JustEnoughVCS/VersionControl/issues page. We will promptly address your feedback.



## License

This project is licensed under the **MIT License**.

For the full license terms, please refer to the ./LICENSE-MIT.md file in the project root directory.

---

Finally, thank you for your interest in `JustEnoughVCS`!
