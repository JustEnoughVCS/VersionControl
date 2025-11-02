<p align="center">
    <a href="https://github.com/JustEnoughVCS/VersionControl">
        <img alt="JustEnoughVCS" src="docs/images/Header_Large.png" width="100%">
    </a>
</p>

<h1 align="center">JustEnoughVCS</h1>

<p align="center">
    Lightweight and Binary-Friendly Centralized Version Control System
</p>

<p align="center">
    <img src="https://img.shields.io/github/stars/JustEnoughVCS/VersionControl?style=for-the-badge">
    <img src="https://img.shields.io/badge/Status-Development%20in%20Progress-yellow?style=for-the-badge">
    <img src="https://img.shields.io/badge/Release-Not%20Available-lightgrey?style=for-the-badge">
</p>

> [!WARNING]
> JustEnoughVCS core features are still under development and not yet usable
>
> If you are interested in our project, we recommend contacting us directly. [Contact and Support](#Support)



## Introduction

JustEnoughVCS simplifies binary collaboration by allowing only one person to edit a file at a time. Its architecture keeps you updated on file status, versions, and other relevant information.

### 1. Personal View

Each member has their own sheet[^sheet] that maps assets to directory structures. This allows personalized file views. Members can focus on their own workspace without worrying about others moving files or affecting others with their own moves.

### 2. Asset "Read" and "Write"

JustEnoughVCS has an intuitive permission model: visible assets can be read, held assets can be modified.

[^sheet]: Sheet: A member's personal file structure. Members can have multiple sheets, but only one can be edited in a local workspace[^local_workspace] at a time.
[^local_workspace]: Local Workspace: The local directory where members edit files.

### 3. Visibility Propagation

To **share an asset with everyone**, export[^export] its mapping to the reference sheet[^ref_sheet]. After vault[^vault] administrator approval, it becomes visible to all members, who can then import[^import] it to receive updates.

To share with **specific sheets**, export the asset visibility to those sheets. After approval from the sheet holders, they can receive your updates.

[^export]: Export: The method for transferring asset mappings in JVCS.
[^import]: Import: The method for obtaining asset mappings in JVCS.
[^ref_sheet]: Reference Sheet: A sheet curated by the vault[^vault] administrator, serving as the team's "asset index".
[^vault]: Vault: The asset repository in JVCS where all assets are stored.

### 4. Editing Rights Transfer

The first member to track an asset becomes its initial holder with full editing rights. To transfer rights, the holder releases them, allowing another member to hold the asset. The latest version is synchronized during this process.



> [!NOTE]
>
> This collaboration model manages binary asset structure and versioning, but is not suitable for program development.
>
> Git already serves that purpose well.



## Roadmap

### Core Library

- [ ] Incremental file updates and storage
- [ ] Multiple reference sheets



### Extension Tools

- [ ] Command Line Tool -> [CommandLine](https://github.com/JustEnoughVCS/CommandLine ) (Currently Private)
- [ ] Declarative Asset Management -> [AssetsConfig](https://github.com/JustEnoughVCS/AssetsConfig) (Currently Private)
- [ ] File Merger -> [BinMerger](https://github.com/JustEnoughVCS/BinMerger) (Currently Private)



## Support

Encountered issues or have suggestions while using JustEnoughVCS?

-   Please submit them to https://github.com/JustEnoughVCS/VersionControl/issues, and we'll address your feedback promptly.

> [!NOTE]
>
> The project is in early development. Instead of creating Issues, we recommend contacting [@Weicao-CatilGrass](https://github.com/Weicao-CatilGrass) directly.
>
> Creating Issues will be more appropriate once basic features are more complete.



## License

This project is licensed under the **MIT License**.

For complete license terms, see ./LICENSE-MIT.md in the project root.

---

Thank you for your interest in `JustEnoughVCS`!
