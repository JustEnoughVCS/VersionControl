# JustEnoughVCS - A Lightweight, Cross-Functional, Binary-Friendly Parallel Version Control System

​	`JustEnoughVCS` is a lightweight version control system designed for **cross-functional teams**. It allows each member to view and organize files in a file structure **best suited to their functional role**, enabling the team to focus on content creation itself. It primarily serves collaborative scenarios involving large volumes of binary assets, such as **game development** and **multimedia design**.

## My Design Philosophy - Also My Humble Opinion 😃

​	`JustEnoughVCS` adheres to the "**Just Enough**" philosophy, aiming to achieve collaborative security through architectural design. Centered around a **Virtual File System** and **Sheet Isolation**, it provides each creator with a focused, distraction-free workspace, making collaboration natural and simple.

## Virtual File System

​	The Virtual File System is the foundation of `JustEnoughVCS`. Each file is identified by a globally unique `VirtualFileId`, decoupled from its physical path. It comprehensively records:

-   **All historical versions**
-   **Description information for each modification**
-   **Version number sequence**
-   **Current latest version**
-   **Current file holder (the member with edit permissions)**

### Features

-   **Traceable History**: Easily view the history of any version and support rollback.
-   **Conflict-Free Collaboration**: Strictly adheres to the **"Acquire First, Edit Later"** principle. Files are **visible** to everyone but **writable** only by the holder, thus preventing conflicts.
-   **Pre-Acquisition Validation**: Before acquiring a file, the hash value and version number of the local file are strictly validated to ensure editing starts from the latest version.

## Sheet System

​	The Sheet System acts as a bridge connecting **virtual files** with members' **local workspaces**. It establishes a mapping from `VirtualFileId` to a local `SheetPath`, creating customized file views for each functional role (e.g., programmer, artist, designer) or individual member.

Sheets are divided into two types, primarily differing in permission management:

-   **Reference Sheet**: Stores files commonly used across the team, serving as a shared resource library. All members can acquire files from it into their own sheets. Members can submit their own files to the Reference Sheet; after approval by an administrator, they are added to the Reference Sheet for other members to import into their own sheets.
-   **Member Sheet**: A member's own sheet, used to manage personal projects, tasks, and assets. The member has full management rights over the **sheet structure** (such as adding, moving mappings, etc.), but **edit rights** for the file itself remain exclusive to the file's holder, following the "Acquire First, Edit Later" principle. Typically, newly tracked files automatically grant ownership to the tracker.

| Operation | Reference Sheet | Member Sheet |
| :--- | :--- | :--- |
| **Check-in** | All Members | All Members |
| **Add Item** | All Members (to a staging area) | Owner |
| **Move Item** | Administrator | Owner |
| **Unlink** | Administrator | Owner |
| **Merge** | Administrator | Owner |
| **Clone** | All Members | All Members |

### Transferring Files Between Sheets

​	Through the **Import/Export mechanism** provided by the Sheet System, members can copy a file's **mapping relationship** from one sheet to another. This mechanism transfers the file's **mapping relationship** (i.e., the binding of `VirtualFileId` to a path), not the file entity itself. The file entity always resides in the Virtual File System. This mechanism supports cross-sheet collaboration, such as submitting completed work to the team for sharing, or recommending a file to a specific member for further processing.

-   **Export**: A member can directly export files from their own sheet to the target sheet's (e.g., Reference Sheet or another member's sheet) pending import area.
-   **Import**: The owner of the target sheet receives a list of files pending import. The owner can review and selectively import them (i.e., add the mapping) into their own sheet. During this process, the file's **edit rights remain unchanged**; it simply makes the file "visible" in the recipient's sheet.

This mechanism ensures the **controllability** of file transfer, as the recipient has the right to decide whether to accept the imported file mapping, thereby maintaining the cleanliness and order of their respective workspaces.

## Support

Encountered any issues or have suggestions while using JustEnoughVCS?

-   Please submit them to the https://github.com/JustEnoughVCS/VersionControl/issues page, and we will promptly address your feedback.

## License

This project is licensed under the **MIT License**.

For the complete license terms, please refer to the ./LICENSE-MIT.md file in the project root directory.

---

Finally, thank you for your interest in `JustEnoughVCS`! 🎉
