use crate::fgpt::{self, AppStateRef, CompletionEvent, CompletionRequest, Message};
use axum::{
    extract::State,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::post,
    Json, Router,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::UNIX_EPOCH,
};

#[derive(Deserialize, Debug, Serialize, Default)]
struct OpenAPIClientRequest {
    messages: Vec<Message>,
    stream: Option<bool>,
}

async fn proxy_completions(
    State(state): State<AppStateRef>,
    Json(params): Json<OpenAPIClientRequest>,
) -> Response {
    log::info!(
        "exec stream:{:?} messages:{:?}",
        params.stream,
        params.messages
    );

    match handle_proxy_completions(State(state), Json(params)).await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("{}", e);
            let resp = Response::new(e.to_string().into());
            let (mut parts, body) = resp.into_parts();
            parts.status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
            Response::from_parts(parts, body)
        }
    }
}

async fn handle_proxy_completions(
    State(state): State<AppStateRef>,
    Json(params): Json<OpenAPIClientRequest>,
) -> Result<Response, fgpt::Error> {
    let stream_mode = params.stream.unwrap_or(false);
    let req = CompletionRequest::new(
        state.clone(),
        params.messages,
        None,
        Some(uuid::Uuid::new_v4().to_string()),
    );

    let mut stream = req.stream(state.clone()).await?;
    if !stream_mode {
        while let Some(Ok(event)) = stream.next().await {
            match event {
                CompletionEvent::Done => {
                    break;
                }
                CompletionEvent::Error(reason) => {
                    return Err(fgpt::Error::Io(reason));
                }
                _ => {}
            }
        }
        let textbuf = stream.textbuf.borrow().clone();
        let body = json!(
            {
                "id": stream.request_id,
                "created": stream
                .start_at
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
                "model": "gpt-3.5-turbo",
                "object": "chat.completion",
                "choices": [
                    {
                        "finish_reason": stream.finish_reason,
                        "index": 0,
                        "message": {
                            "content": textbuf,
                            "role": "assistant"
                        }
                    }
                ],
                "usage": {
                    "prompt_tokens": stream.prompt_tokens,
                    "completion_tokens": stream.completion_tokens,
                    "total_tokens": stream.total_tokens()
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
            "sync exec request_id:{} elapsed:{:.2}s throughput:{:.2} tokens:{}",
            stream.request_id,
            stream.start_at.elapsed().unwrap().as_secs_f64(),
            *stream.completion_tokens.borrow() as f64
                / stream.start_at.elapsed().unwrap().as_secs_f64(),
            stream.total_tokens()
        );

        return Ok(Response::from_parts(parts, body.into()));
    }
    return Ok(Sse::new(CompletionToSSEStream { stream }).into_response());
}
struct CompletionToSSEStream {
    stream: fgpt::CompletionStream,
}

impl Stream for CompletionToSSEStream {
    type Item = reqwest::Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = &mut self.stream;
        let poll = stream.poll_next_unpin(cx);
        match poll {
            Poll::Ready(Some(Ok(event))) => match event {
                CompletionEvent::Data(data) => {
                    let body = json!(
                        {
                            "id": stream.request_id,
                            "created":stream
                            .start_at
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                            "model": "gpt-3.5-turbo",
                            "object": "chat.completion.chunk",
                            "choices": [
                                {
                                    "index": 0,
                                    "finish_reason": stream.finish_reason,
                                    "delta": {
                                        "content": data.delta_chars,
                                        "role": "assistant"
                                    }
                                }
                            ],
                        }
                    );
                    let event = Event::default().data(body.to_string());
                    Poll::Ready(Some(Ok(event)))
                }
                CompletionEvent::Done => {
                    let completion_tokens = *stream.completion_tokens.borrow();
                    let total_tokens = completion_tokens + stream.prompt_tokens;
                    log::info!(
                        "async exec request_id:{} elapsed:{:.2}s throughput:{:.2} tokens:{}",
                        stream.request_id,
                        stream.start_at.elapsed().unwrap().as_secs_f64(),
                        completion_tokens as f64 / stream.start_at.elapsed().unwrap().as_secs_f64(),
                        total_tokens
                    );
                    Poll::Ready(None)
                }
                CompletionEvent::Error(reason) => {
                    let body = json!(
                        {
                            "id": stream.request_id,
                            "created": stream.start_at.duration_since(UNIX_EPOCH).unwrap().as_secs_f64(),
                            "model": "gpt-3.5-turbo",
                            "object": "chat.completion.chunk",
                            "choices": [
                                {
                                    "index": 0,
                                    "finish_reason": "error",
                                    "delta": {
                                        "content": reason,
                                    }
                                }
                            ],
                        }
                    );
                    let event = Event::default().data(body.to_string());
                    Poll::Ready(Some(Ok(event)))
                }
                _ => Poll::Pending,
            },
            Poll::Ready(Some(Err(event))) => Poll::Ready(Some(Err(event))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub async fn serve(state: AppStateRef) -> Result<(), fgpt::Error> {
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
