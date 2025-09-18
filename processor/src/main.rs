use anyhow::Result;
use clap::Parser;
use std::{collections::HashMap, path::Path};
use tempfile::NamedTempFile;
use tokio::{fs, io::AsyncWriteExt};

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

    println!("Processing {}", args.file);

    // TODO: Insert Wit_World import in the python file:
    // import wit_world
    // class WitWorld(wit_world.WitWorld):

    match build_python_component(&[&args.file]).await {
        Ok(bytes) => {
            println!("Component built successfully, {} bytes.", bytes.len());
            let mut file = fs::File::create("component.wasm").await?;
            file.write_all(&bytes).await?;
        }
        Err(e) => {
            eprintln!("Failed to build component: {}", e);
        }
    }

    // Compose the components together using wac

    // Generate the Spin app

    Ok(())
}

async fn build_python_component(src_paths: &[&str]) -> Result<Vec<u8>> {
    let tmp = NamedTempFile::new()?;

    // Stub wasi interfaces using wasi-virt (only support WASI@0.2.1, and componentize_py only
    // supports WASI@0.2.0), hence just stubbing WASI for now.

    componentize_py::componentize(
        Some(Path::new("./test/wit")),
        Some("stringprocessor"),
        &[],
        false,
        None,
        src_paths,
        &[],
        "python",
        tmp.path(),
        None,
        true,
        &HashMap::new(),
        &HashMap::new(),
    )
    .await?;
    Ok(fs::read(tmp.path()).await?)
}
