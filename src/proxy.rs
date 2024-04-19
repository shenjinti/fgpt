use crate::{fgpt::CompletionRequest, StateRef};
use axum::{
    extract::State,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::post,
    Json, Router,
};
use futures::StreamExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Deserialize, Debug, Serialize, Default)]
struct OpenAPIClientRequest {
    messages: Vec<crate::fgpt::Message>,
    stream: bool,
}

async fn proxy_completions(
    State(state): State<StateRef>,
    Json(params): Json<OpenAPIClientRequest>,
) -> Response {
    let completion_id = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(28)
        .map(char::from)
        .collect::<String>();

    let request_id = format!("chatcmpl-{}", completion_id);
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let tokenizer = gpt_tokenizer::Default::new();

    let prompt_tokens: usize = params
        .messages
        .iter()
        .map(|message| tokenizer.encode(&message.content).len())
        .sum();

    log::debug!(
        "exec request_id:{} stream:{} messages:{:?}",
        request_id,
        params.stream,
        params.messages
    );

    let start_at = std::time::Instant::now();

    if params.stream {
        let r = match crate::fgpt::execute_plain(
            state.clone(),
            params.messages,
            None,
            Some(uuid::Uuid::new_v4().to_string()),
            |_| async move {},
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!(
                    "Failed to sync execute_plain: {} request_id:{} ",
                    e.to_string(),
                    request_id
                );
                let resp = Response::new(e.to_string().into());
                let (mut parts, body) = resp.into_parts();
                parts.status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                return Response::from_parts(parts, body);
            }
        };

        let completion_tokens = tokenizer.encode(&r.textbuf).len();
        let total_tokens = completion_tokens + prompt_tokens;
        let body = json!(
            {
                "id": request_id,
                "created": created_at,
                "model": "gpt-3.5-turbo",
                "object": "chat.completion",
                "choices": [
                    {
                        "finish_reason": r.finish_reason,
                        "index": 0,
                        "message": {
                            "content": r.textbuf,
                            "role": "assistant"
                        }
                    }
                ],
                "usage": {
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": total_tokens
                }
            }
        );
        let resp = Response::new(body.to_string());
        let (mut parts, body) = resp.into_parts();

        parts.status = axum::http::StatusCode::OK;
        parts.headers.insert(
            "content-type",
            axum::http::HeaderValue::from_static("application/json"),
        );
        log::info!(
            "sync exec request_id:{} elapsed:{}s throughput:{} tokens:{}",
            request_id,
            start_at.elapsed().as_secs_f64(),
            completion_tokens as f64 / start_at.elapsed().as_secs_f64(),
            total_tokens
        );
        return Response::from_parts(parts, body.into());
    } else {
        let (tx, rx) = unbounded_channel::<Result<Event, Infallible>>();
        let sse_stream = UnboundedReceiverStream::new(rx);

        tokio::spawn(async move {
            let req = CompletionRequest::new(
                state.clone(),
                params.messages,
                None,
                Some(uuid::Uuid::new_v4().to_string()),
            );
            let mut stream = match req.stream(state.clone()).await {
                Ok(stream) => stream,
                Err(e) => {
                    log::error!(
                        "Failed to async execute_plain: {} request_id:{} ",
                        e.to_string(),
                        request_id
                    );
                    return;
                }
            };
            let mut textbuf = String::new();
            let mut finish_reason = None;

            while let Some(message) = stream.next().await {
                match message {
                    Ok(crate::fgpt::CompletionEvent::Data(message)) => {
                        if message.message.author.role != "assistant" {
                            continue;
                        }

                        let text = message.message.content.parts.join("\n");
                        if textbuf.len() > text.len() {
                            continue;
                        }
                        finish_reason = message.get_finish_reason();
                        let delta_chars = &text[textbuf.len()..];
                        let body = json!(
                            {
                                "id": request_id,
                                "created": created_at,
                                "model": "gpt-3.5-turbo",
                                "object": "chat.completion.chunk",
                                "choices": [
                                    {
                                        "index": 0,
                                        "finish_reason": finish_reason,
                                        "delta": {
                                            "content": delta_chars,
                                            "role": "assistant"
                                        }
                                    }
                                ],
                            }
                        );
                        let event = Event::default().data(body.to_string());
                        if tx.send(Ok(event)).is_err() {
                            break;
                        }
                        textbuf = text.clone();
                    }
                    Ok(crate::fgpt::CompletionEvent::Done) => {
                        let body = json!(
                            {
                                "id": request_id,
                                "created": created_at,
                                "model": "gpt-3.5-turbo",
                                "object": "chat.completion.chunk",
                                "choices": [
                                    {
                                        "index": 0,
                                        "finish_reason": finish_reason,
                                        "delta": {
                                            "content": "",
                                        }
                                    }
                                ],
                            }
                        );
                        let event = Event::default().data(body.to_string());
                        tx.send(Ok(event)).ok();
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{:?}", e);
                        break;
                    }
                }
            }
        });

        return Sse::new(sse_stream).into_response();
    }
}

pub async fn serve(state: crate::StateRef) -> Result<(), crate::fgpt::Error> {
    let app = Router::new()
        .nest(
            &state.prefix,
            Router::new().route("/chat/completions", post(proxy_completions)),
        )
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.serve_addr).await?;
    //
    println!("free GPT-3.5 cli tools | 🪐 https://github.com/shenjinti/fgpt");
    println!("💖 To star the repository if you like \x1b[1;32mfgpt\x1b[0m!");
    println!();
    println!("🚀 Server is running at http://{}", state.serve_addr);
    println!("Base URL: http://{}/v1", state.serve_addr);
    println!("Endpoint: http://{}/v1/chat/completions", state.serve_addr);

    axum::serve(listener, app).await.map_err(|e| e.into())
}
