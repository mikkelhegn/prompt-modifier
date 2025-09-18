use anyhow::Result;
use clap::Parser;
use std::{collections::HashMap, path::Path, str::FromStr};
use tokio::{
    fs,
    io::{self, AsyncWriteExt},
};
use uuid::Uuid;
use wac_graph::{CompositionGraph, EncodeOptions, types::Package};

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

    let files_path = Path::new(&args.file);

    match files_path.extension() {
        Some(ext) if ext == "wasm" => {
            println!("This is a .wasm file, going straight to composition");

            // Generate the composed component using WAC
            let composed_component = compose_components(files_path.to_str().expect("Coudl not extract wasm file path")).await;
            
            println!("Result: {}", composed_component.unwrap());
            Ok(())
        }
        Some(ext) if ext == "py" => {
            println!("This is a .py file, let's prep, compile and compose.");
            // Prep the Python app. Wrap the two module before and after in a Class compatible with exporting the WIT interfaces defined
            let prepped_app = prep_app_modules(
                &String::from_str(files_path.parent().unwrap().to_str().unwrap())?,
                uuid,
            )
            .await?;

            // Create a component out of the Python application
            let component = build_python_component(&prepped_app.as_str(), uuid).await;

            // Generate the composed component using WAC
            let composed_component = compose_components(component.unwrap().as_str()).await;

            println!("Result: {}", composed_component.unwrap());
            Ok(())
        }
        Some(_) | None => {
            println!("No file extension found");
            Ok(())
        }
    }
}

async fn prep_app_modules(directory: &String, uuid: Uuid) -> Result<String> {
    let template_path = "python_template_modules.liquid";
    let template = fs::read_to_string(template_path)
        .await
        .expect("Failed to read teamplaet file");

    // Build imports
    let imports = format!(
        "from {} import before\nfrom {} import after",
        "before", "after"
    );

    // Replace {{ imports }}
    let output_file_content = template.replace("{{ imports }}", &imports);

    // Save on temp disk with UUID-based path
    let path_string = format!("temp/{}/component-input/app.py", uuid);
    let output_file_path = Path::new(&path_string);
    save_to_disk(output_file_path, output_file_content.as_bytes())
        .await
        .expect("Failed to save file");

    // Copy all modules
    copy_dir_contents(
        directory,
        output_file_path
            .parent()
            .unwrap()
            .to_str()
            .expect("Failed to get parent dir"),
    )
    .await?;

    // TODO return file content as bytes
    Ok(path_string)
}

async fn build_python_component(src_paths: &str, uuid: Uuid) -> Result<String> {
    // TODO: install dependencies
    // requirements.py

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
        &HashMap::new(),
    )
    .expect("Failed to genereate bindings");

    // componentize
    let component_path_string = format!("temp/{}/component-output/component.wasm", uuid);
    let component_output_file_path = Path::new(&component_path_string);
    fs::create_dir_all(component_output_file_path.parent().unwrap())
        .await
        .expect("Failed to create output dir");

    let python_app_directory = Path::new(src_paths);
    println!("Python app directory {:?}", python_app_directory);

    componentize_py::componentize(
        Some(Path::new("../shared/wit")),
        Some("promptmodifier"),
        &[],
        false,
        None,
        &[python_app_directory
            .parent()
            .unwrap()
            .to_str()
            .expect("Failed to find parent direcoty of the Python app")],
        &[],
        "app",
        component_output_file_path,
        None,
        true,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await
    .expect("Failed to generate Python component");

    Ok(component_path_string)
}

async fn compose_components(component_path: &str) -> Result<String> {
    // WAC the components together...
    // wac plug --plug ../examples/python-component/app.wasm target/wasm32-wasip2/release/temp_goal_rust.wasm -o composed.wasm
    // save to (temp/{GUID}/spin-app-output)
    let spin_template_app_component_path =
        Path::new("../spin-app-template/target/wasm32-wasip2/release/temp_goal_rust.wasm");
    assert!(!Path::new("does_not_exist.txt").exists());
    wac_it(Path::new(component_path), spin_template_app_component_path);

    Ok("WAC'ed it!".to_string())
}

fn wac_it(prompt_modifier_path: &Path, http_handler_path: &Path) {
    println!("Prompt Modifier: {:?}", prompt_modifier_path);
    println!("HTTP Handler: {:?}", http_handler_path);

    let mut graph = CompositionGraph::new();

    // Register the package dependencies into the graph
    let package = Package::from_file("app", None, prompt_modifier_path, graph.types_mut()).unwrap();
    let prompt_modifier = graph.register_package(package).unwrap();
    let package = Package::from_file("host", None, http_handler_path, graph.types_mut()).unwrap();
    let http_handler = graph.register_package(package).unwrap();

    // Instantiate the prompt modifier instance which does not have any arguments
    let prompt_modifier_instance = graph.instantiate(prompt_modifier);

    // Instantiate the http handler instance which has a single argument "promptmodification" which is exported by the prompt modifier instance
    let http_handler_instance = graph.instantiate(http_handler);
    let prompt_modifier_export = graph
        .alias_instance_export(
            prompt_modifier_instance,
            "component:promptprocessor/promptmodification@0.0.1",
        )
        .unwrap();
    graph
        .set_instantiation_argument(
            http_handler_instance,
            "component:promptprocessor/promptmodification@0.0.1",
            prompt_modifier_export,
        )
        .unwrap();

    // Alias the http handler export from the grehttp handler instance
    let http_handler_export = graph
        .alias_instance_export(http_handler_instance, "wasi:http/incoming-handler@0.2.0")
        .unwrap();
    // Export the "greet" function from the composition
    graph
        .export(http_handler_export, "wasi:http/incoming-handler@0.2.0")
        .unwrap();

    // Encode the graph into a WASM binary
    let encoding = graph.encode(EncodeOptions::default()).unwrap();
    std::fs::write("../spin-app-template/composed.wasm", encoding).unwrap();
}

// Helper functions
async fn copy_dir_contents(src: &str, dst: &str) -> io::Result<()> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    // Ensure destination directory exists
    fs::create_dir_all(dst_path).await?;

    let mut entries = fs::read_dir(src_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_name = entry.file_name();
            let dst_file = dst_path.join(file_name);
            fs::copy(&path, &dst_file).await?;
        }
    }

    Ok(())
}

async fn save_to_disk(
    output_file_path: &Path,
    output_file_content: &[u8],
) -> Result<(), anyhow::Error> {
    fs::create_dir_all(
        output_file_path
            .parent()
            .expect("Could not extract paretn dir"),
    )
    .await
    .expect("Failed to create directory");
    let mut output_file = fs::File::create(output_file_path)
        .await
        .expect("Failed to create output file");
    output_file
        .write_all(output_file_content)
        .await
        .expect("Feild to write to file");
    Ok(())
}
