mod deps;

use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn handle_string_processor_http(req: Request) -> anyhow::Result<impl IntoResponse> {
    println!("Handling request to {:?}", req.header("spin-full-url"));

    let string = deps::component_stringprocessor::component::stringprocessor::stringprocessing::processstring("hello");

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(string)
        .build())
}
