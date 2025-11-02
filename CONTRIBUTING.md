# Welcome to Contributing!

**Before You Begin**:

Thank you for your interest in and support of JustEnoughVCS! If you wish to contribute to the project, please read this guide carefully first.

We warmly welcome and value every contributor's submission, but to ensure a smooth and efficient contribution process, please follow the standardized approach.



## 1. Choose Your Module

`JustEnoughVCS` uses a modular architecture that separates core functionality from client tools into different repositories:

| Repository | Link                                                         | Description                  |
| ---------- | ------------------------------------------------------------ | ---------------------------- |
| Core Logic | [Version Control](https://github.com/JustEnoughVCS/VersionControl) | Main logic, asset management |
| Frontend   |                                                              |                              |
| CLI        | [Command Line](https://github.com/JustEnoughVCS/CommandLine) | Glue code between core and CLI |

Please first understand which module of `JustEnoughVCS` you want to contribute to - this is important!

1. If you want to optimize version control logic, add or modify core features, please go to `VersionControl`.

2. If you want to optimize, modify, or add logic for frontend interfaces like command-line or GUI tools, please go to the corresponding repository.



## 2. Set Up the Project

Fork the repository you want to modify to your GitHub account, then clone it locally using SSH.

For non-`VersionControl` parts, please also clone the core repository into a sibling directory to ensure it can reference the `VersionControl` repository.

Structure should look like this:

```
.
├── <frontend-name>    # Frontend & Extensions
│   ├── src/
│   ├── Cargo.toml
│   └── README.md
└── VersionControl/    # Core library
    ├── src/
    ├── Cargo.toml
    ├── CONTRIBUTING.md
    ├── LICENSE-MIT.md
    └── README.md
```



> [!NOTE]
>
> We currently don't use `git submodule` because the core library and frontends require extensive synchronized modifications
>
> This will transition to `git submodule` approach once the project stabilizes



## 3. Set Up Development Environment

Development environment configuration can be found in the documentation of each repository.

`JustEnoughVCS` uses different technology stacks for different frontend directions.

For example: The `CommandLine` part uses Rust + Clap to build the command-line program, while the `MyVault` GUI uses Avalonia + .NET.



## 4. Submit Your PR

Before submitting, please ensure:

1. Create a feature branch in your forked repository, and develop based on the repository's `dev` or `docs` branch
2. Implement your feature or fix
3. Describe your changes in the COMMIT message and push to your fork
4. Create a Pull Request to the main repository's `dev` or `docs` branch



### Note: PRs We Cannot Accept

Changes we don't expect:

1. Major changes that haven't been discussed in the Issues section
2. Features that increase rather than reduce mental complexity

Wrong merge branches:

1. Documentation changes merged to the `dev` branch
2. Code changes merged to the `docs` branch
3. Changes merged to the `main` branch



## Open Source Licenses

Different `JustEnoughVCS` projects use different open source licenses. For example, the current `VersionControl` uses the MIT License (see LICENSE-MIT.md file), while the GUI part uses the `GPLv3` license.



Finally, thank you for your support of `JustEnoughVCS`!
