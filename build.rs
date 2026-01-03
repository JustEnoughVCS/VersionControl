use std::env;
use std::path::PathBuf;
use std::process::Command;

const COMPILE_INFO_RS: &str = "./src/data/compile_info.rs";
const COMPILE_INFO_RS_TEMPLATE: &str = "./src/data/compile_info.rs.template";

fn main() {
    println!("cargo:rerun-if-env-changed=FORCE_BUILD");

    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    if let Err(e) = generate_compile_info(&repo_root) {
        eprintln!("Failed to generate compile info: {}", e);
        std::process::exit(1);
    }
}

/// Generate compile info
fn generate_compile_info(repo_root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Read the template code
    let template_code = std::fs::read_to_string(repo_root.join(COMPILE_INFO_RS_TEMPLATE))?;

    let date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let platform = get_platform(&target);
    let toolchain = get_toolchain();
    let version = get_version();
    let branch = get_git_branch().unwrap_or_else(|_| "unknown".to_string());
    let commit = get_git_commit().unwrap_or_else(|_| "unknown".to_string());

    let generated_code = template_code
        .replace("{date}", &date)
        .replace("{target}", &target)
        .replace("{platform}", &platform)
        .replace("{toolchain}", &toolchain)
        .replace("{version}", &version)
        .replace("{branch}", &branch)
        .replace("{commit}", &commit);

    // Write the generated code
    let compile_info_path = repo_root.join(COMPILE_INFO_RS);
    std::fs::write(compile_info_path, generated_code)?;

    Ok(())
}

fn get_platform(target: &str) -> String {
    if target.contains("windows") {
        "Windows".to_string()
    } else if target.contains("linux") {
        "Linux".to_string()
    } else if target.contains("darwin") || target.contains("macos") {
        "macOS".to_string()
    } else if target.contains("android") {
        "Android".to_string()
    } else if target.contains("ios") {
        "iOS".to_string()
    } else {
        "Unknown".to_string()
    }
}

fn get_toolchain() -> String {
    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let channel = if rustc_version.contains("nightly") {
        "nightly"
    } else if rustc_version.contains("beta") {
        "beta"
    } else {
        "stable"
    };

    format!("{} ({})", rustc_version, channel)
}

fn get_version() -> String {
    let cargo_toml_path = std::path::Path::new("Cargo.toml");
    let cargo_toml_content = match std::fs::read_to_string(cargo_toml_path) {
        Ok(content) => content,
        Err(_) => return "unknown".to_string(),
    };

    let cargo_toml: toml::Value = match toml::from_str(&cargo_toml_content) {
        Ok(value) => value,
        Err(_) => return "unknown".to_string(),
    };

    if let Some(workspace) = cargo_toml.get("workspace") {
        if let Some(package) = workspace.get("package") {
            if let Some(version) = package.get("version") {
                if let Some(version_str) = version.as_str() {
                    return version_str.to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

/// Get current git branch
fn get_git_branch() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout)?.trim().to_string();

        if branch.is_empty() {
            // Try to get HEAD reference if no branch (detached HEAD)
            let output = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()?;

            if output.status.success() {
                let head_ref = String::from_utf8(output.stdout)?.trim().to_string();
                return Ok(head_ref);
            }
        } else {
            return Ok(branch);
        }
    }

    Err("Failed to get git branch".into())
}

/// Get current git commit hash
fn get_git_commit() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;

    if output.status.success() {
        let commit = String::from_utf8(output.stdout)?.trim().to_string();
        return Ok(commit);
    }

    Err("Failed to get git commit".into())
}
