# JustEnoughVCS

​	`JustEnoughVCS` is a lightweight version control system designed for **cross-functional teams**. It moves away from traditional single-directory tree constraints, allowing each member to view and organize files in a way that **best suits their functional role**, enabling teams to focus on content creation itself. It primarily serves collaborative scenarios rich in binary assets, such as **game development** and **multimedia design**.

## Virtual File System

​	The Virtual File System is the foundation of `JustEnoughVCS`. Each file is identified by a globally unique `VirtualFileId`, decoupled from its physical path. It comprehensively records:

-   **All historical versions**
-   **Description information for each modification**
-   **Version number sequence**
-   **Current latest version**
-   **Current file holder (the member with editing permissions)**

### Features

-   **Traceable History**: Easily view the history of any version and support rollbacks.
-   **Conflict-Free Collaboration**: Strictly adheres to the **"acquire before edit"** principle. Files are **visible** to everyone but **writable** only by the holder, preventing conflicts.
-   **Pre-Acquisition Validation**: Before acquiring a file, the local file's hash value and version number are strictly validated to ensure editing begins from the latest version.

## Sheet System

​	The Sheet System acts as a bridge connecting **virtual files** to a member's **local workspace**. It establishes mappings from `VirtualFileId` to local `SheetPath`, creating customized file views for each functional role (e.g., programmer, artist, designer) or individual member.

Sheets are divided into two types, differing primarily in permission management:

| Operation | Reference Struct Sheet | Member Struct Sheet |
| :--- | :--- | :--- |
| **Check-in** | All Members | All Members |
| **Add Item** | All Members (enters a temporary area) | Owner |
| **Move Item** | Administrator | Owner |
| **Break Link** | Administrator | Owner |
| **Merge** | Administrator | Owner |
| **Clone** | All Members | All Members |

## Support

Encounter any issues or have suggestions while using JustEnoughVCS?

-   Please submit them to the https://github.com/JustEnoughVCS/VersionControl/issues page, and we will address your feedback promptly.

## License

This project is licensed under the **MIT License**.

For the complete license terms, please see the ./LICENSE-MIT.md file in the root directory of the project.

---

Finally, thank you for your interest in `JustEnoughVCS`! 🎉
