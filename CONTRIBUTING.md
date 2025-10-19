# JustEnoughVCS Contribution Guide

Welcome to the JustEnoughVCS (referred to as `JVCS`) project! Before you start contributing, please read this guide carefully to ensure a smooth contribution process.

## Project Structure

JustEnoughVCS adopts a modular architecture that separates core functionality from frontend tools into different repositories:

### Core Repositories
- **Core Library**: https://github.com/JustEnoughVCS/VersionControl
- **Command Line Frontend**: https://github.com/JustEnoughVCS/CommandLine

## Contribution Process

### 1. Preparation

**Core Library Contribution**:
- Fork the core repository to your GitHub account
- Clone it locally using SSH

**Frontend Contribution**:
- Fork the corresponding frontend repository
- Clone it locally using SSH

### 2. Project Directory Structure

All frontend projects follow a unified directory structure:

```
.
├── <Frontend Name>
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

### 3. Development Environment Setup

1. Ensure you have the Rust toolchain installed
2. Clone the project locally
3. Configure the development environment according to the README instructions in each repository

### 4. Submitting a Pull Request

1. Create a feature branch in your forked repository
2. Implement your feature or fix
3. Write appropriate test cases
4. Ensure your code passes all existing tests
5. Write clear commit messages
6. Create a Pull Request to the main repository

## Important Notes

- Please ensure your code follows the project's coding standards
- Before submitting a PR, sync with the latest changes from the main repository
- For significant changes, it's recommended to discuss the design approach in Issues first

Thank you for contributing to the JustEnoughVCS project!
