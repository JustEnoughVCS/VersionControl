// Auto-generated code.

/// From ./docs/Documents/readmes/local_workspace_todolist.md
pub const READMES_LOCAL_WORKSPACE_TODOLIST: &str = "# Setup a Workspace

You have created an empty local **Workspace** using `jv create` or `jv init`.

At this point, the workspace exists only as a local structure.
It is **not yet associated with any identity or upstream Vault**.

Follow the steps below to complete the initial setup.

## Account Setup

> [!TIP]
> If you have already registered an account on this machine, you can skip this section.

An account represents **this machine acting on your behalf**.
You can create an account and generate a private key using the following commands.

```bash
# Make sure OpenSSL is installed on your system.
# Create an account and automatically generate a private key using OpenSSL.
jv account add pan --keygen

# If you already have a private key, you can associate it with an account.
jv account movekey pan ./your_private_key.pem
```

After creating the private key, generate the corresponding public key.
This also requires `OpenSSL` support.

```bash
# Generate the public key in the current directory.
jv account genpub pan ./
```

**Send the generated public key file to a Host of the upstream Vault.**
Only after the key is registered by the Vault can this workspace authenticate.

## Login

> In the following example, we assume:
>
> * the account name is `pan`
> * the upstream Vault address is `127.0.0.1`

Navigate to the workspace directory (the directory containing this file), then run:

```bash
# Log in as pan to the upstream Vault at 127.0.0.1
# -C skips confirmation prompts
jv login pan 127.0.0.1 -C
```

This command performs the following steps internally:

```bash
jv account as pan        # Bind the workspace to account pan
jv direct 127.0.0.1 -C   # Set the upstream Vault address
jv update                # Fetch initial structure and metadata from upstream
rm SETUP.md              # Remove this setup document
```

After login, the workspace becomes a **live participant** connected to the upstream Vault.

## Completion

At this point, the workspace is fully set up:

* An account identity is bound locally
* An upstream Vault is configured
* Initial data has been synchronized

Normally, `jv login` removes this file automatically.
If the file still exists due to a deletion failure, please remove it manually to keep the workspace clean.

Once this file is gone, the workspace is considered **ready for daily use**.

## Why does this file delete itself?

A freshly created JVCS workspace is considered **clean** only when no sheets are in use and no unexplained files exist.

During the initial setup phase, this `SETUP.md` file is the only allowed exception.
It exists solely to guide the workspace into a connected and explainable state.

Once the workspace is connected to an upstream Vault and enters normal operation,
every file in the workspace is expected to be **explainable by structure**.

If this setup file remains:

* it becomes an unexplained local file
* `JustEnoughVCS` cannot determine which sheet it belongs to
* and any subsequent `jv use` operation would be ambiguous

To prevent this ambiguity, JVCS enforces a strict rule:

**A workspace with unexplained files is not allowed to enter active use.**

Deleting `SETUP.md` marks the end of the setup phase and confirms that:

* all remaining files are intentional
* their placement can be explained
* and the workspace is ready to participate in structure-driven operations

This is why the setup document removes itself.
It is not cleanup — it is a boundary.";

/// From ./docs/Documents/readmes/vault_readme.md
pub const READMES_VAULT_README: &str = "# Setup an Upstream Vault

Thank you for using `JustEnoughVCS`.

This document guides you through setting up an **Upstream Vault** — a shared source of structure and content that other workspaces may depend on.

## Configuration File

Before starting the Vault service with `jvv listen`, you need to configure `vault.toml`.

This file defines the identity, connectivity, and responsibility boundaries of the Vault.

### Adding Hosts

Set the `hosts` parameter to specify which members are allowed to switch into the **Host** role for this Vault.

```toml
# Pan and Alice are allowed to operate this Vault as hosts
hosts = [ \"pan\", \"alice\" ]
```

Members listed here can switch their identity as follows:

```bash
jv as host/pan
```

> Becoming a Host means operating from a position that may affect other members.
> This role is not a permanent identity, but a responsibility-bearing context.

### Configuring Connection

Modify the `bind` parameter to `0.0.0.0` to allow access from other devices on the local network.

```toml
bind = \"0.0.0.0\"
```

> [!TIP]
> `JustEnoughVCS` uses port **25331** by default.
> You can change the listening port by modifying the `port` parameter if needed.

### Disabling Logger (Optional)

If you prefer a cleaner console output, you can disable logging:

```toml
logger = \"no\"
```

## Member Account Setup

Run the following command in the root directory of the **Vault** to register a member:

```bash
jvv member register pan
```

Registering a member only creates its identity record in the Vault.
It does **not** automatically make the member login-ready.

An authentication method must be registered separately.

> [!NOTE]
> Currently, `JustEnoughVCS` supports **key-based authentication** only.

Place the public key file provided by the member (`name.pem`) into the Vault’s `./key` directory.

For example, after registering a member named `pan`, the Vault directory structure should look like this:

```
.
├── key
│   └── pan.pem      <- Public key file
├── members
│   ├── pan.bcfg     <- Member registration info
│   └── host.bcfg
├── README.md
├── sheets
│   └── ref.bcfg
├── storage
└── vault.toml
```

## Completion

At this point, the Vault is fully configured:

* Member identities are registered
* Host roles are defined
* Authentication materials are in place

You can now start the Vault service using:

```bash
jvv listen
```

Other workspaces may connect to this Vault once it is listening.";

/// From ./docs/Documents/ASCII_YIZI.txt
pub const ASCII_YIZI: &str = "#BANNER START#
  ████████                ████████
██▒▒▒▒▒▒▒▒██            ██▒▒▒▒▒▒▒▒██
██        ▒▒██        ██▒▒        ██    █████  ██    ██   ██████   █████
██          ▒▒████████▒▒          ██    ▒▒▒██  ██    ██   ██████  ██████
██            ▒▒▒▒▒▒▒▒            ██       ██  ██    ██  ███▒▒▒█  █▒▒▒▒█
██                                ██       ██  ██    ██  ███   ▒  ████ ▒
██                                ██       ██  ██    ██  ███      ▒████
██         ████      ████         ██       ██  ▒██  ██▒  ███       ▒▒▒██
██         ████      ████         ██    █  ██   ██  ██   ███   █  ██  ██
██         ████      ████         ██    █  ██   ▒████▒   ▒██████  ██████
██         ▒▒▒▒      ▒▒▒▒   █     ██    ▒████    ▒██▒     ██████  ▒████▒
██                         ██     ██     ▒▒▒▒     ▒▒      ▒▒▒▒▒▒   ▒▒▒▒
██                ██████████      ██
██                                ██    {banner_line_1}
  ████████████████████████████████      {banner_line_2}
  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒      {banner_line_3}
#BANNER END#";

/// From ./docs/Documents/README.md
pub const README: &str = "# Documents
Documents of JustEnoughVCS";

/// From ./docs/Documents/profiles/vault.toml
pub const PROFILES_VAULT: &str = "#
#
#    ████████                ████████
#  ██▒▒▒▒▒▒▒▒██            ██▒▒▒▒▒▒▒▒██
#  ██        ▒▒██        ██▒▒        ██    █████  ██    ██  ██    ██
#  ██          ▒▒████████▒▒          ██    ▒▒▒██  ██    ██  ██    ██
#  ██            ▒▒▒▒▒▒▒▒            ██       ██  ██    ██  ██    ██
#  ██                                ██       ██  ██    ██  ██    ██
#  ██                                ██       ██  ██    ██  ██    ██
#  ██         ████      ████         ██       ██  ▒██  ██▒  ▒██  ██▒
#  ██         ████      ████         ██    █  ██   ██  ██    ██  ██
#  ██         ████      ████         ██    █  ██   ▒████▒    ▒████▒
#  ██         ▒▒▒▒      ▒▒▒▒   █     ██    ▒████    ▒██▒      ▒██▒
#  ██                         ██     ██     ▒▒▒▒     ▒▒        ▒▒
#  ██                ██████████      ██
#  ██                                ██
#    ████████████████████████████████     JustEnoughVCS Vault Profile
#    ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒     ===========================
#
#

# Vault \"{vault_name}\" created by {user_name} ({date_format})

#########################
### Base Informations ###
#########################
name = \"{vault_name}\"
uuid = \"{vault_uuid}\" # Don't touch it!

# Administrator list, indicating members who can switch identity to 'host'
hosts = []

[profile]

####################
### Connectivity ###
####################

# Bind address: 127.0.0.1 for localhost only, 0.0.0.0 to allow remote access
bind = \"127.0.0.1\" # (fallback: \"127.0.0.1\")

# Bind port
port = 25331 # (fallback: 25331)

# Enable LAN discovery: clients can discover this service on the local network
# TIP: Not yet supported
lan_discovery = \"disable\" # (enable, disable, fallback: disable)

#############
### Debug ###
#############

# Enable logger
logger = \"yes\" # (yes, no, fallback: no)

# Logger output level
logger_level = \"info\" # (debug, trace, info, fallback: info)

#################################
### Authentication & Security ###
#################################

# Authentication mode
# TIP: Currently only \"key\" is supported
auth_mode = \"key\" # (key, password, noauth, fallback: password)";

/// From ./docs/Documents/docs/collaboration.txt
pub const DOCS_COLLABORATION: &str = "NO CONTENT YET :(";

/// From ./docs/Documents/docs/get_started.txt
pub const DOCS_GET_STARTED: &str = "NO CONTENT YET :(";

// Get document content by name
pub fn document(name: impl AsRef<str>) -> Option<String> {
    match name.as_ref() {
        "readmes_local_workspace_todolist" => Some(READMES_LOCAL_WORKSPACE_TODOLIST.to_string()),
        "readmes_vault_readme" => Some(READMES_VAULT_README.to_string()),
        "ascii_yizi" => Some(ASCII_YIZI.to_string()),
        "readme" => Some(README.to_string()),
        "profiles_vault" => Some(PROFILES_VAULT.to_string()),
        "docs_collaboration" => Some(DOCS_COLLABORATION.to_string()),
        "docs_get_started" => Some(DOCS_GET_STARTED.to_string()),
_ => None,
    }
}

// Get list of all available document names
pub fn documents() -> Vec<String> {
    vec![
        "readmes_local_workspace_todolist".to_string(),
        "readmes_vault_readme".to_string(),
        "ascii_yizi".to_string(),
        "readme".to_string(),
        "profiles_vault".to_string(),
        "docs_collaboration".to_string(),
        "docs_get_started".to_string(),
    ]
}
