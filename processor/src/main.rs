use anyhow::{Ok, Result, anyhow};
use clap::{Parser, ValueEnum};
use glob::glob;
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs;
use uuid::Uuid;
use wac_graph::{CompositionGraph, EncodeOptions, types::Package};

const COMPONENT_INPUT_FOLDER_NAME: &str = "component-input";
const COMPONENT_OUTPUT_FOLDER_NAME: &str = "component-output";
const COMPONENT_FILE_NAME: &str = "component.wasm";

#[derive(Clone)]
struct Job {
    id: Uuid,
    source_path: PathBuf,
    language: Option<CodeLanguages>,
    temp_dir: PathBuf,
}

impl Job {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            source_path: PathBuf::new(),
            language: None,
            temp_dir: PathBuf::new(),
        }
    }
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum CodeLanguages {
    Python,
    JavaScript,
    TypeScript,
    Wasm,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Source, component or directory with app
    #[arg(
        short,
        long,
        help = "Use this to provide the path to your code project directory or a component."
    )]
    source_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut job = Job::new();

    job.source_path = PathBuf::from_str(args.source_path.as_str())
        .expect(format!("Failed to parse source argument: {:?}", args.source_path).as_str());

    job.language = match figure_language(&job) {
        Some(l) => {
            println!("I think this is a {:?} component / program", l);
            Some(l)
        }
        None => return Err(anyhow!("Language not supported")),
    };

    let mut job_steps = 1;
    let mut current_step = 0;

    if job
        .clone()
        .language
        .is_some_and(|l| l != CodeLanguages::Wasm)
    {
        job_steps = 3
    };

    job.temp_dir = env::temp_dir().join(job.id.to_string());

    println!("Processing {:?}, as job-id: {:?}", job.source_path, job.id);

    println!("Temporary directory: {:?}", job.temp_dir);

    // Checks what steps to take given the input file that was provided
    if job
        .language
        .clone()
        .is_some_and(|l| l != CodeLanguages::Wasm)
    {
        // Prep the app
        current_step += 1;
        println!(
            "{}/{}: Preparing the {:?} app",
            current_step, job_steps, job.language
        );
        prep_app_modules(&job).await?;
        println!("Done");

        // Create a component out of the application
        current_step += 1;
        println!("{}/{}: Creating the component", current_step, job_steps);
        build_component(&job).await?;
        println!("Done");
    }

    // Composing the components together
    current_step += 1;
    println!("{}/{}: Creating the component", current_step, job_steps);
    compose_components(&job)
        .await
        .expect("Failed to compose components");
    println!("Done!");

    Ok(())
}

/// Heuristics to define the programming language
fn figure_language(job: &Job) -> Option<CodeLanguages> {
    match job.source_path.extension() {
        Some(ext) => match OsStr::to_ascii_lowercase(ext).to_str() {
            Some("wasm") => {
                return Some(CodeLanguages::Wasm);
            }
            _ => {
                println!(
                    "Only `.wasm` files supported. It looks like you didn't proivde a `.wasm` file, or a directory as the source."
                );
                return None;
            }
        },
        None => {
            let path = job.source_path.to_str().unwrap();
            if glob(format!("{path}/**/*.py").as_str())
                .expect("No files in the directory")
                .count()
                > 0
            {
                return Some(CodeLanguages::Python);
            } else if glob(format!("{path}/**/*.js").as_str())
                .expect("No files in the directory")
                .count()
                > 0
            {
                return Some(CodeLanguages::JavaScript);
            } else {
                println!("Didn't find a supported code language.");
                return None;
            }
        }
    }
}

/// Takes a Job as argument and build a component for the job
async fn build_component(job: &Job) -> Result<()> {
    match job.language {
        Some(CodeLanguages::Python) => {
            return build_python_component(job).await;
        }
        Some(CodeLanguages::JavaScript) | Some(CodeLanguages::TypeScript) => {
            todo!("JavaScript or TypeScript not supported - yet!")
        }
        _ => panic!("No programming language provided"),
    }
}

/// Takes a Job as argument and prepares the application
async fn prep_app_modules(job: &Job) -> Result<()> {
    // Check programming language
    match job.language {
        Some(CodeLanguages::Python) => {
            // Copy source app to temp directory
            copy_dir_contents(
                job.source_path.clone(),
                job.temp_dir.join("component-input"),
            )
            .await?;

            // Copy python module wrapper to temp directory
            let pyhton_module_file = include_bytes!("../includes/python_module.py");
            fs::write(
                job.temp_dir.join("component-input/python_module.py"),
                pyhton_module_file,
            )
            .await
            .expect("Failed to copy Python mpdule file");

            Ok(())
        }
        Some(CodeLanguages::JavaScript) | Some(CodeLanguages::TypeScript) => {
            todo!("JavaScript or TypeScript not supported - yet!")
        }
        _ => panic!("No programming language provided"),
    }
}

/// Takes a Job object and returns a Result
async fn build_python_component(job: &Job) -> Result<()> {
    // TODO: Support installing dependencies defined in requirements.py

    let wit_file = include_bytes!("../includes/world.wit");
    fs::write(job.temp_dir.join("world.wit"), wit_file)
        .await
        .expect("Failed to copy world.wit file.");

    // Generating bindings
    componentize_py::generate_bindings(
        &[job.temp_dir.join("world.wit")],
        Some("promptmodifier"),
        &[],
        false,
        None,
        job.temp_dir
            .join(format!("{}/promptmodifier", COMPONENT_INPUT_FOLDER_NAME))
            .as_path(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .expect("Failed to genereate bindings");

    // Create the component
    fs::create_dir_all(job.temp_dir.join(COMPONENT_OUTPUT_FOLDER_NAME))
        .await
        .expect("Failed to create output dir");

    componentize_py::componentize(
        &[job.temp_dir.join("world.wit")],
        Some("promptmodifier"),
        &[],
        false,
        None,
        &[job
            .temp_dir
            .join(COMPONENT_INPUT_FOLDER_NAME)
            .to_str()
            .ok_or("Failed to parse component temp folder")
            .unwrap()],
        &[],
        "python_module",
        &job.temp_dir
            .join(COMPONENT_OUTPUT_FOLDER_NAME)
            .join(COMPONENT_FILE_NAME),
        None,
        true,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await
    .expect("Failed to generate Python component");

    Ok(())
}

/// WAC the components together
async fn compose_components(job: &Job) -> Result<()> {
    let prompt_modifier_path = job
        .temp_dir
        .join(COMPONENT_OUTPUT_FOLDER_NAME)
        .join(COMPONENT_FILE_NAME);

    if job
        .language
        .as_ref()
        .is_some_and(|l| l == &CodeLanguages::Wasm)
    {
        if !(prompt_modifier_path.exists()) {
            fs::create_dir_all(prompt_modifier_path.clone().parent().unwrap())
                .await
                .expect(
                    format!("Failed to create dir: {:?}", prompt_modifier_path.clone()).as_str(),
                );
            fs::copy(job.source_path.clone(), prompt_modifier_path.clone())
                .await
                .expect(
                    format!(
                        "Failed to copy source wasm: {:?} to: {:?}",
                        job.source_path,
                        prompt_modifier_path.clone()
                    )
                    .as_str(),
                );
        };
    };

    println!("Prompt Modifier: {:?}", prompt_modifier_path);

    let mut graph = CompositionGraph::new();

    // Register the package dependencies into the graph
    let user_package =
        Package::from_file("app", None, prompt_modifier_path, graph.types_mut()).unwrap();
    let prompt_modifier = graph.register_package(user_package).unwrap();

    let http_handler_file = include_bytes!("../includes/temp_goal_rust.wasm");
    let host_package =
        Package::from_bytes("host", None, http_handler_file, graph.types_mut()).unwrap();
    let http_handler = graph.register_package(host_package).unwrap();

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
    // TODO: Enable choice of output location
    std::fs::write(PathBuf::from("./composed.wasm"), encoding)
        .expect("Failed to write compose wasm file");

    Ok(())
}

// Helper functions

/// Copy all files from `src` to `dst`. Creates the `dst` directory (and all parent directories), if they do not exist.
async fn copy_dir_contents(src: PathBuf, dst: PathBuf) -> Result<()> {
    let src_path = Path::new(&src);
    let dst_path = Path::new(&dst);

    fs::create_dir_all(dst_path)
        .await
        .expect(format!("Failed to create directory: {:?}", dst_path).as_str());

    let mut entries = fs::read_dir(src_path)
        .await
        .expect(format!("Failed to read directory: {:?}", src_path).as_str());

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_name = entry.file_name();
            let dst_file = dst_path.join(file_name);
            fs::copy(&path, &dst_file)
                .await
                .expect(format!("Failed to copy file: {:?} to: {:?} ", &path, &dst_file).as_str());
        }
    }

    Ok(())
}
