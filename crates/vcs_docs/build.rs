use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

// Template markers for code generation
const TEMPLATE_DOCUMENT_BEGIN: &str = "--- TEMPLATE DOCUMENT BEGIN ---";
const TEMPLATE_DOCUMENT_END: &str = "--- TEMPLATE DOCUMENT END ---";
const TEMPLATE_FUNC_BEGIN: &str = "--- TEMPLATE FUNC BEGIN ---";
const TEMPLATE_FUNC_END: &str = "--- TEMPLATE FUNC END ---";
const TEMPLATE_LIST_BEGIN: &str = "--- TEMPLATE LIST BEGIN ---";
const TEMPLATE_LIST_END: &str = "--- TEMPLATE LIST END ---";

// Template parameter patterns for substitution
const PARAM_DOCUMENT_PATH: &str = "{{DOCUMENT_PATH}}";
const PARAM_DOCUMENT_CONSTANT_NAME: &str = "{{DOCUMENT_CONSTANT_NAME}}";
const PARAM_DOCUMENT_CONTENT: &str = "{{DOCUMENT_CONTENT}}";
const PARAM_DOCUMENT_PATH_SNAKE_CASE: &str = "{{DOCUMENT_PATH_SNAKE_CASE}}";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/docs.rs.template");
    println!("cargo:rerun-if-changed=../../docs/Documents");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("docs.rs");

    // Read all markdown files from docs directory recursively
    let docs_dir = Path::new("../../docs/Documents");
    let mut documents = Vec::new();

    if docs_dir.exists() {
        collect_text_files(docs_dir, &mut documents)?;
    }

    // Read template file
    let template_path = Path::new("src/docs.rs.template");
    let template_content = fs::read_to_string(template_path)?;

    // Extract template sections preserving original indentation
    let document_template = template_content
        .split(TEMPLATE_DOCUMENT_BEGIN)
        .nth(1)
        .and_then(|s| s.split(TEMPLATE_DOCUMENT_END).next())
        .unwrap_or("")
        .trim_start_matches('\n')
        .trim_end_matches('\n');

    let match_arm_template = template_content
        .split(TEMPLATE_FUNC_BEGIN)
        .nth(1)
        .and_then(|s| s.split(TEMPLATE_FUNC_END).next())
        .unwrap_or("")
        .trim_start_matches('\n')
        .trim_end_matches('\n');

    // Generate document blocks and match arms
    let mut document_blocks = String::new();
    let mut match_arms = String::new();
    let mut list_items = String::new();

    for (relative_path, content) in &documents {
        // Calculate parameters for template substitution
        let document_path = format!("./docs/Documents/{}", relative_path);

        // Generate constant name from relative path
        let document_constant_name = relative_path
            .replace(['/', '\\', '-'], "_")
            .replace(".md", "")
            .replace(".txt", "")
            .replace(".toml", "")
            .replace(".yaml", "")
            .replace(".yml", "")
            .replace(".json", "")
            .replace(".rs", "")
            .to_uppercase();

        // Generate snake_case name for function matching
        let document_path_snake_case = relative_path
            .replace(['/', '\\', '-'], "_")
            .replace(".md", "")
            .replace(".txt", "")
            .replace(".toml", "")
            .replace(".yaml", "")
            .replace(".yml", "")
            .replace(".json", "")
            .replace(".rs", "")
            .to_lowercase();

        // Escape double quotes in content
        let escaped_content = content.trim().replace('\"', "\\\"");

        // Replace template parameters in document block preserving indentation
        let document_block = document_template
            .replace(PARAM_DOCUMENT_PATH, &document_path)
            .replace(PARAM_DOCUMENT_CONSTANT_NAME, &document_constant_name)
            .replace(PARAM_DOCUMENT_CONTENT, &escaped_content)
            .replace("r#\"\"#", &format!("r#\"{}\"#", escaped_content));

        document_blocks.push_str(&document_block);
        document_blocks.push_str("\n\n");

        // Replace template parameters in match arm preserving indentation
        let match_arm = match_arm_template
            .replace(PARAM_DOCUMENT_PATH_SNAKE_CASE, &document_path_snake_case)
            .replace(PARAM_DOCUMENT_CONSTANT_NAME, &document_constant_name);

        match_arms.push_str(&match_arm);
        match_arms.push('\n');

        // Generate list item for documents() function
        let list_item = format!("        \"{}\".to_string(),", document_path_snake_case);
        list_items.push_str(&list_item);
        list_items.push('\n');
    }

    // Remove trailing newline from the last list item
    if !list_items.is_empty() {
        list_items.pop();
    }

    // Build final output by replacing template sections
    let mut output = String::new();

    // Add header before document blocks
    if let Some(header) = template_content.split(TEMPLATE_DOCUMENT_BEGIN).next() {
        output.push_str(header.trim());
        output.push_str("\n\n");
    }

    // Add document blocks
    output.push_str(&document_blocks);

    // Add function section
    if let Some(func_section) = template_content.split(TEMPLATE_FUNC_BEGIN).next()
        && let Some(rest) = func_section.split(TEMPLATE_DOCUMENT_END).nth(1)
    {
        output.push_str(rest.trim());
        output.push('\n');
    }

    // Add match arms
    output.push_str(&match_arms);

    // Add list items for documents() function
    if let Some(list_section) = template_content.split(TEMPLATE_LIST_BEGIN).next()
        && let Some(rest) = list_section.split(TEMPLATE_FUNC_END).nth(1)
    {
        output.push_str(rest.trim());
        output.push('\n');
    }
    output.push_str(&list_items);

    // Add footer
    if let Some(footer) = template_content.split(TEMPLATE_LIST_END).nth(1) {
        // Preserve original indentation in footer
        output.push_str(footer);
    }

    // Write generated file
    let mut file = fs::File::create(&dest_path)?;
    file.write_all(output.as_bytes())?;

    // Copy to src directory for development
    let src_dest_path = Path::new("src/docs.rs");
    fs::write(src_dest_path, output)?;

    Ok(())
}

fn collect_text_files(dir: &Path, documents: &mut Vec<(String, String)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_text_files(&path, documents)?;
        } else if path.extension().is_some_and(|ext| {
            ext == "md"
                || ext == "txt"
                || ext == "toml"
                || ext == "yaml"
                || ext == "yml"
                || ext == "json"
                || ext == "rs"
        }) && let Ok(relative_path) = path.strip_prefix("../../docs/Documents")
            && let Some(relative_path_str) = relative_path.to_str()
        {
            let content = fs::read_to_string(&path)?;
            documents.push((
                relative_path_str.trim_start_matches('/').to_string(),
                content,
            ));
        }
    }
    Ok(())
}
