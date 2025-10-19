# Welcome to Contributing!

Welcome to contributing to `JustEnoughVCS`! Before you begin, please read this guide to ensure a smooth contribution process.



## 1. Choose Your Direction

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

For non-`VersionControl` parts, please also clone the core repository using HTTP into a sibling directory to ensure it can reference the `VersionControl` repository.

Structure should look like this:

```
.
├── <frontend-name>
│   ├── src/           # Frontend source code
│   ├── Cargo.toml     # Rust project configuration
│   └── README.md      # Project documentation
└── VersionControl/    # Core library reference
    ├── src/
    ├── Cargo.toml
    ├── CONTRIBUTING.md
    ├── LICENSE-MIT.md
    └── README.md
```



## 3. Set Up Development Environment

Development environment configuration can be found in the documentation of each repository.

`JustEnoughVCS` uses different technology stacks for different frontend directions.

For example: The `CommandLine` part uses Rust + Clap to build the command-line program, while the `MyVault` GUI uses Avalonia + .NET.



## 4. Submit Your PR

Before submitting, please ensure:

1. Create a feature branch in your forked repository
2. Implement your feature or fix
3. Write appropriate test cases and pass all tests
4. Submit clear commit messages
5. Create a Pull Request to the main repository

### Notes

- Ensure your code follows the project's coding standards
- Sync with the latest changes from the main repository before submitting PR
- For major changes, it's recommended to discuss the design approach in Issues first



## Finally, Open Source Licenses

Different `JustEnoughVCS` projects use different open source licenses. For example, the current `VersionControl` uses the very permissive MIT License (see LICENSE-MIT.md file), while `MyVault` uses the `GPLv3` license.



Thank you for your support of `JustEnoughVCS`!
