use crate::rgpt::{CompletionRequest, Message};
use futures::StreamExt;
use std::io::Write;

pub async fn run(state: crate::StateRef) -> Result<(), crate::rgpt::Error> {
    let mut messages = vec![];
    match state.qusetion {
        Some(ref q) => {
            messages.push(Message {
                role: "user".to_string(),
                content: q.clone(),
                content_type: "text".to_string(),
            });
        }
        None => {}
    }

    let req = CompletionRequest::new(state.clone(), messages);
    let mut stream = req.stream(state.clone()).await?;
    let mut textbuf = String::new();

    while let Some(message) = stream.next().await {
        match message {
            Ok(crate::rgpt::CompletionEvent::Data(message)) => {
                let text = message.message.content.parts.join("\n");
                let delta_chars = text.strip_prefix(textbuf.as_str()).unwrap_or(text.as_str());
                textbuf = text.clone();
                print!("{}", delta_chars);
                let _ = std::io::stdout().flush();
            }
            Ok(crate::rgpt::CompletionEvent::Done) => {
                println!();
                log::debug!("End of conversation");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                log::info!("Error: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}
