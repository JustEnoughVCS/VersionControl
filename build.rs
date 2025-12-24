use std::env;
use std::path::PathBuf;

const COMPILE_INFO_RS: &str = "./src/data/compile_info.rs";
const COMPILE_INFO_RS_TEMPLATE: &str = "./src/data/compile_info.rs.template";

fn main() {
    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Only generate installer script on Windows
    if cfg!(target_os = "windows") {
        if let Err(e) = generate_installer_script(&repo_root) {
            eprintln!("Failed to generate installer script: {}", e);
            std::process::exit(1);
        }
    }

    if let Err(e) = generate_compile_info(&repo_root) {
        eprintln!("Failed to generate compile info: {}", e);
        std::process::exit(1);
    }
}

/// Generate Inno Setup installer script (Windows only)
fn generate_installer_script(repo_root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let template_path = repo_root.join("setup/windows/setup_jv_cli_template.iss");
    let output_path = repo_root.join("setup/windows/setup_jv_cli.iss");

    let template = std::fs::read_to_string(&template_path)?;

    let author = get_author()?;
    let version = get_version();
    let site = get_site()?;

    let generated = template
        .replace("<<<AUTHOR>>>", &author)
        .replace("<<<VERSION>>>", &version)
        .replace("<<<SITE>>>", &site);

    std::fs::write(output_path, generated)?;
    Ok(())
}

fn get_author() -> Result<String, Box<dyn std::error::Error>> {
    let cargo_toml_path = std::path::Path::new("Cargo.toml");
    let cargo_toml_content = std::fs::read_to_string(cargo_toml_path)?;
    let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content)?;

    if let Some(package) = cargo_toml.get("package") {
        if let Some(authors) = package.get("authors") {
            if let Some(authors_array) = authors.as_array() {
                if let Some(first_author) = authors_array.get(0) {
                    if let Some(author_str) = first_author.as_str() {
                        return Ok(author_str.to_string());
                    }
                }
            }
        }
    }

    Err("Author not found in Cargo.toml".into())
}

fn get_site() -> Result<String, Box<dyn std::error::Error>> {
    let cargo_toml_path = std::path::Path::new("Cargo.toml");
    let cargo_toml_content = std::fs::read_to_string(cargo_toml_path)?;
    let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content)?;

    if let Some(package) = cargo_toml.get("package") {
        if let Some(homepage) = package.get("homepage") {
            if let Some(site_str) = homepage.as_str() {
                return Ok(site_str.to_string());
            }
        }
    }

    Err("Homepage not found in Cargo.toml".into())
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

    let generated_code = template_code
        .replace("{date}", &date)
        .replace("{target}", &target)
        .replace("{platform}", &platform)
        .replace("{toolchain}", &toolchain)
        .replace("{version}", &version);

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
