use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use spin_sdk::llm::{self};

#[http_component]
async fn handle_temp_goal_rust(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let prompt = "Tell me a joke";

    match llm::infer(llm::InferencingModel::Llama2Chat, prompt) {
        Ok(resp) => {
            let resp = Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(format!("{:?}", resp.text))
                .build();
            println!("{resp:?}");
            Ok(resp)
        }
        Err(resp) => {
            let resp = Response::builder()
                .status(500)
                .header("content-type", "text/plain")
                .body(format!("Inferencing failed! {:?}", resp))
                .build();
            println!("{resp:?}");
            Ok(resp)
        }
    }
}
