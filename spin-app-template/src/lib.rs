use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use spin_sdk::llm::{self};

use crate::component::promptprocessor::promptmodification;

wit_bindgen::generate!({
    world: "promptmodifierhost",
    path: "../shared/wit/world.wit"
});

#[http_component]
async fn handle_temp_goal_rust(req: Request) -> anyhow::Result<impl IntoResponse> {

    let userprompt = match req.body().len() {
        0 => {
            "You're a standup comedian. Tell us a joke about developers, by completing the following: Once there was a developer..."
        }
        _ => {
            &String::from_utf8_lossy(req.body())
        }
    };

    println!("User Prompt: {:?}", userprompt);

    let post_before_prompt = promptmodification::before(userprompt);

    println!("Modified user Prompt: {:?}", post_before_prompt);

    match llm::infer(
        llm::InferencingModel::Llama2Chat,
        post_before_prompt.as_str(),
    ) {
        Ok(resp) => {
            println!("LLM Response: {:?}", &resp.text);
            let post_after_prompt = promptmodification::after(&resp.text);
            println!("Response after modifier: {:?}", post_after_prompt);
            let resp = Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(format!("{:?}", post_after_prompt))
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
