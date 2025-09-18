use anyhow::Result;
use clap::Parser;
use std::{collections::HashMap, path::Path};
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;
use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use regex::Regex;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the python file
    #[arg(short, long)]
    file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let uuid = Uuid::new_v4();

    println!("Processing {}, as job-id: {:?}", args.file, uuid);

    let prepped_app = prep_app(&args.file, uuid).await?;

    let component = build_python_component(&prepped_app.as_str(), uuid).await;

    let spin_app = generate_spin_app(component.unwrap().as_str()).await;

    // TODO let _test_app = test_component();
    // test?

    // TODO let _test_spin_app = test_spin_app();
    // test?

    // TODO let _deploy_spin_app = deploy_spin_app();
    // deploy

    println!("Result: {}", spin_app.unwrap());
    Ok(())
}

async fn generate_spin_app(component_path: &str) -> Result<String> {
    // WAC the components together...
    // wac plug --plug ../examples/python-component/app.wasm target/wasm32-wasip2/release/temp_goal_rust.wasm -o composed.wasm
    // save to (temp/{GUID}/spin-app-output)
    let spin_template_app_component_path = Path::new("../spin-app-template/target/wasm32-wasip2/release/temp_goal_rust.wasm");
    assert!(!Path::new("does_not_exist.txt").exists());
    wac_it(Path::new(component_path), spin_template_app_component_path);

    Ok("WAC'ed it!".to_string())
}

async fn build_python_component(src_paths: &str, uuid: Uuid) -> Result<String> {

    // TODO: install dependencies

    // TODO generate bindings...
    // componentize-py -d ../../shared/wit/world.wit -w promptmodifier bindings promptmodifier

    let bindings_path_string = format!("temp/{}/component-input/promptmodifier", uuid);
    let bindings_output_file_path = Path::new(&bindings_path_string);

    componentize_py::generate_bindings(
        Path::new("../shared/wit"),
        Some("promptmodifier"),
        &[],
        false,
        None,
        bindings_output_file_path,
        &HashMap::new(),
        &HashMap::new()
    ).expect("Failed to genereate bindings");

    // componentize
    let component_path_string = format!("temp/{}/component-output/component.wasm", uuid);
    let component_output_file_path = Path::new(&component_path_string);
    fs::create_dir_all(component_output_file_path.parent().unwrap()).await.expect("Failed to create output dir");

    let python_app_directory = Path::new(src_paths);
    println!("Python app directory {:?}", python_app_directory);

    componentize_py::componentize(
        Some(Path::new("../shared/wit")),
        Some("promptmodifier"),
        &[],
        false,
        None,
        &[python_app_directory.parent().unwrap().to_str().expect("Failed to find parent direcoty of the Python app")],
        &[],
        "app",
        component_output_file_path,
        None,
        true,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await.expect("Failed to generate Python component");

    // TODO: Stub wasi interfaces using wasi-virt (only support WASI@0.2.1, and componentize_py only
    // supports WASI@0.2.0), hence just stubbing WASI for now.

    // Return the output path
    // TODO return the file conetnt as bytes
    Ok(component_path_string)
}

async fn prep_app(file: &String, uuid: Uuid) -> Result<String> {
    let source_path = file;
    let template_path = "python_template.liquid";

    let source_file = fs::read_to_string(source_path.as_str()).await.expect("Failed to read source file");
    let template_file = fs::read_to_string(template_path).await.expect("Failed to read template file");

    let before_body = extract_function_body(&source_file, "before")
        .expect("Could not find function `before` in source file");
    let after_body = extract_function_body(&source_file, "after")
        .expect("Could not find function `after` in source file");

    // Map parameter name replacements:
    let before_body = replace_ident(&before_body, "prompt", "userprompt");
    let after_body = replace_ident(&after_body, "prompt", "promptresult");

    // Re-indent to 8 spaces (matches template indentation in your example)
    let before_indented = indent_lines(&before_body, 8);
    let after_indented = indent_lines(&after_body, 8);

    let output_file_content = template_file
        .replace("{{ before_code }}", &before_indented)
        .replace("{{ after_code }}", &after_indented);

    // Save on temp disk with UUID-based path
    let path_string = format!("temp/{}/component-input/app.py", uuid);
    let output_file_path = Path::new(&path_string);
    save_to_disk(output_file_path, output_file_content.as_bytes()).await.expect("Failed to save file");

    // TODO return file content as bytes
    Ok(path_string)
}

async fn save_to_disk(output_file_path: &Path, output_file_content: &[u8]) -> Result<(), anyhow::Error> {
    fs::create_dir_all(output_file_path.parent().expect("Could not extract paretn dir")).await.expect("Failed to create directory");
    let mut output_file = fs::File::create(output_file_path).await.expect("Failed to create output file");
    output_file.write_all(output_file_content).await.expect("Feild to write to file");
    Ok(())
}

fn wac_it(prompt_modifier_path: &Path, http_handler_path: &Path) {

    println!("Prompt Modifier: {:?}", prompt_modifier_path);
    println!("HTTP Handler: {:?}", http_handler_path);

    let mut graph = CompositionGraph::new();

    // Register the package dependencies into the graph
    let package = Package::from_file(
        "app",
        None,
        prompt_modifier_path,
        graph.types_mut(),
    )
    .unwrap();
    let prompt_modifier = graph.register_package(package).unwrap();
    let package = Package::from_file(
        "host",
        None,
        http_handler_path,
        graph.types_mut(),
    )
    .unwrap();
    let http_handler = graph.register_package(package).unwrap();

    // Instantiate the prompt modifier instance which does not have any arguments
    let prompt_modifier_instance = graph.instantiate(prompt_modifier);

    // Instantiate the http handler instance which has a single argument "promptmodification" which is exported by the prompt modifier instance
    let http_handler_instance = graph.instantiate(http_handler);
    let prompt_modifier_export = graph
        .alias_instance_export(prompt_modifier_instance, "component:promptprocessor/promptmodification@0.0.1")
        .unwrap();
    graph
        .set_instantiation_argument(http_handler_instance, "component:promptprocessor/promptmodification@0.0.1", prompt_modifier_export)
        .unwrap();

    // Alias the http handler export from the grehttp handler instance
    let http_handler_export = graph
        .alias_instance_export(http_handler_instance, "wasi:http/incoming-handler@0.2.0")
        .unwrap();
    // Export the "greet" function from the composition
    graph.export(http_handler_export, "wasi:http/incoming-handler@0.2.0").unwrap();

    // Encode the graph into a WASM binary
    let encoding = graph.encode(EncodeOptions::default()).unwrap();
    std::fs::write("../spin-app-template/composed.wasm", encoding).unwrap();

}


/// Extract the body of a top-level `def <name>(...)` function from python source.
/// Returns dedented body (no leading indentation).
fn extract_function_body(source: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    // find line that starts with "def name("
    let def_pattern = format!(r"^def\s+{}\s*\(", regex::escape(name));
    let def_re = Regex::new(&def_pattern).unwrap();

    let mut idx_opt = None;
    for (i, l) in lines.iter().enumerate() {
        if def_re.is_match(l.trim_start()) {
            idx_opt = Some(i);
            break;
        }
    }
    let start_idx = idx_opt?;

    // collect following lines that are indented (body) or blank lines after def
    let mut body_lines: Vec<&str> = Vec::new();
    let mut i = start_idx + 1;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            // keep blank lines in body
            body_lines.push(line);
            i += 1;
            continue;
        }
        // If the next non-empty line starts with whitespace -> part of body
        let first_char = line.chars().next().unwrap_or('\0');
        if first_char.is_whitespace() {
            body_lines.push(line);
            i += 1;
            continue;
        }
        // otherwise we've reached next top-level statement (not indented) -> stop
        break;
    }

    if body_lines.is_empty() {
        return Some("    pass".to_string()); // or empty?
    }

    // compute minimal indent of non-empty lines
    let mut min_indent = usize::MAX;
    for l in &body_lines {
        if l.trim().is_empty() {
            continue;
        }
        let count = l.chars().take_while(|c| c.is_whitespace()).count();
        if count < min_indent {
            min_indent = count;
        }
    }
    if min_indent == usize::MAX {
        min_indent = 0;
    }

    // dedent
    let dedented = body_lines
        .into_iter()
        .map(|l| {
            if l.trim().is_empty() {
                "".to_string()
            } else {
                l.chars().skip(min_indent).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(dedented)
}

/// Replace simple identifier occurrences (word boundaries) of `from_ident` with `to_ident`.
/// NOTE: this is a naive textual substitution using regex word boundaries and DOES NOT
/// understand Python strings or comments. For simple function bodies (like your example),
/// it works fine.
fn replace_ident(body: &str, from_ident: &str, to_ident: &str) -> String {
    let pat = Regex::new(&format!(r"\b{}\b", regex::escape(from_ident))).unwrap();
    pat.replace_all(body, to_ident).to_string()
}

/// Indent each line with `n_spaces` spaces. Blank lines remain blank.
fn indent_lines(body: &str, n_spaces: usize) -> String {
    let indent = " ".repeat(n_spaces);
    body
        .lines()
        .map(|l| {
            if l.trim().is_empty() {
                "".to_string()
            } else {
                format!("{}{}", indent, l)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}