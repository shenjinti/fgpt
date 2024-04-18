use std::{collections::HashMap, fmt, pin::Pin, task::{Context, Poll}};
use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Local};
use futures::stream::Stream;
use reqwest::{header::{ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, ORIGIN, PRAGMA, REFERER, USER_AGENT}, Client, Proxy};
use rustyline::error::ReadlineError;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use crate::StateRef;

const OPENAI_ENDPOINT: &str = "https://chat.openai.com";
const OPENAI_API_URL: &str = "https://chat.openai.com/backend-anon/conversation";
const OPENAI_SENTINEL_URL: &str = "https://chat.openai.com/backend-anon/sentinel/chat-requirements";

#[derive(Debug)]
pub enum Error {
    Io(String),
    Reqwest(String),
    Serde(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e.to_string())
    }
}

#[cfg(feature = "cli")]
impl From<ReadlineError> for Error{
    fn from(e: ReadlineError) -> Self {
        match e {
            ReadlineError::Eof => Error::Io("EOF".to_string()),
            _ => Error::Io(e.to_string()),
        }
    }
    
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            Error::Serde(e) => write!(f, "Serde error: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct Session {
    pub start_at: std::time::Instant,
    pub token: String,
    pub device_id: String,
}
#[derive(serde::Deserialize)]
struct ChatRequirementsResponse {
    pub token: String,
}

#[derive(Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub content_type: String,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Message", 2)?;
        s.serialize_field("author", 
            &serde_json::json!({
                "role": &self.role
            })
        )?;
        s.serialize_field("content", &serde_json::json!({
            "content_type": &self.content_type,
            "parts": &serde_json::json!(vec![&self.content]),
        }))?;
        s.end()
    }
}

#[derive(Default, Serialize)]
pub struct CompletionRequest {
    pub action:String,
    pub model:String,
    pub messages:Vec<Message>,
    pub conversation_mode:HashMap<String, String>,
    pub websocket_request_id: String,
    pub conversation_id:Option<String>,
    pub parent_message_id:Option<String>,
    pub timezone_offset_min:i32,
    pub history_and_training_disabled:bool,
}

impl CompletionRequest {
    pub fn new(state: StateRef, messages:Vec<Message>, conversation_id:Option<String>, parent_message_id:Option<String>, ) -> Self {
        let local: DateTime<Local> = Local::now();
        let offset_minutes = local.offset().local_minus_utc() / 60;

        Self {
            action: "next".to_string(),
            messages,
            model: state.model.clone(),
            conversation_mode: {
                let mut map = HashMap::new();
                map.insert("kind".to_string(), "primary_assistant".to_string());
                map},
            websocket_request_id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            parent_message_id,
            timezone_offset_min: offset_minutes,
            history_and_training_disabled:false,
        }
    }
    
    pub async fn stream(&self, state:StateRef) -> Result<CompletionStream, Error> {
        let start_at = std::time::Instant::now();
        let session = alloc_session(state.clone()).await?;
        let builder = build_req(OPENAI_API_URL, &session.device_id, Some(&session.token), state.clone())?;
        let body = serde_json::to_string(&self)?;
        let resp = builder.body(body.clone()).send().await?;

        log::debug!("open stream: {} ms, proxy: {:?} -> {:?}", start_at.elapsed().as_millis(), state.proxy, resp.status());

        if !resp.status().is_success() {
            let resp_body = resp.text().await?;
            return Err(Error::Reqwest(resp_body));
        }
        Ok(CompletionStream { 
            response_stream: Box::pin(resp.bytes_stream()),
            buffer: BytesMut::new()
         })
    }
}

#[derive(Debug)]
pub(crate) enum CompletionEvent {
    Data(CompletionResponse),
    Done,
    Heartbeat,
    #[allow(unused)]
    Text(String),
}

impl From<&BytesMut> for CompletionEvent {
    fn from(line: &BytesMut) -> CompletionEvent {
        let line_str = String::from_utf8_lossy(&line).to_string();
        let line_str = line_str.strip_prefix("data: ").unwrap_or(&line_str);

        if line_str == "[DONE]" {
            return CompletionEvent::Done
        }
        let heartbeat_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}.\d{6}$").unwrap();
        if heartbeat_re.is_match(line_str) {
            CompletionEvent::Heartbeat
        } else {
            serde_json::from_str(line_str).map(CompletionEvent::Data).unwrap_or(CompletionEvent::Text(line_str.to_string()))
        }
    }
}
#[allow(unused)]
#[derive(Debug, Deserialize)]
pub(crate) struct CompletionMessageAuthor {
    pub role:String,
    pub name:Option<String>,
    pub metadata:HashMap<String, serde_json::Value>,
}
#[allow(unused)]
#[derive(Debug, Deserialize)]
pub(crate) struct CompletionMessageContent {
    pub content_type:String,
    pub parts:Vec<String>,
}
#[allow(unused)]
#[derive(Debug, Deserialize)]
pub(crate) struct CompletionMessageMeta {
    pub citations:Option<Vec<String>>,
    pub gizmo_id:Option<String>,
    pub message_type:Option<String>,
    pub model_slug:Option<String>,
    pub default_model_slug:Option<String>,
    pub pad:Option<String>,
    pub parent_id:Option<String>,
    pub model_switcher_deny:Option<Vec<String>>,
    pub is_visually_hidden_from_conversation:Option<bool>,
}
#[allow(unused)]
#[derive(Debug, Deserialize)]
pub(crate) struct CompletionMessage {
    pub id:String,
    pub author:CompletionMessageAuthor,
    pub create_time:Option<f64>,
    pub update_time:Option<f64>,
    pub content:CompletionMessageContent,
    pub status:String,
    pub end_turn:Option<bool>,
    pub weight:Option<f64>,
    pub metadata:CompletionMessageMeta,
    pub recipient:String,

}
#[allow(unused)]
#[derive(Debug, Deserialize)]
pub(crate) struct CompletionResponse {
    pub message:CompletionMessage,
    pub conversation_id:String,
    pub error:Option<String>,
}

pub(crate) struct CompletionStream {
    response_stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: BytesMut,
}


impl Stream for CompletionStream {
    type Item = reqwest::Result<CompletionEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.response_stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(data))) => {
                    self.buffer.extend_from_slice(&data);
                    if let Some(pos) = self.buffer.windows(2).position(|window| window == b"\n\n") {
                        let mut line = self.buffer.split_to(pos + 2);
                        line.truncate(pos);
                        return Poll::Ready(Some(Ok(CompletionEvent::from(&line))));
                    }
                },
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => {
                    if !self.buffer.is_empty() {
                        if let Some(pos) = self.buffer.windows(2).position(|window| window == b"\n\n") {
                            let mut line = self.buffer.split_to(pos + 2);
                            line.truncate(pos);
                            return Poll::Ready(Some(Ok(CompletionEvent::from(&line))));
                        }
                    } else {
                        return Poll::Ready(None);
                    }
                },
                Poll::Pending => continue,
            }
        }
    }
}

fn build_req(url:&str, device_id:&str, token:Option<&str>, state:StateRef) -> Result<reqwest::RequestBuilder, reqwest::Error> {
    let client = match state.proxy.as_ref() {
        Some(proxy) => {
            let proxy = Proxy::all(proxy)?;
            Client::builder().proxy(proxy).build()?
        }
        None => Client::new(),
    };

    let builder = client
        .post(url)    
        .header("oai-language", state.lang.clone())
        .header("oai-device-id", device_id)
        .header(ACCEPT, "*/*")
        .header(
            ACCEPT_LANGUAGE,
            format!("{},en;q=0.9", state.lang.clone()),
        )
        .header(CACHE_CONTROL, "no-cache")
        .header(PRAGMA, "no-cache")
        .header(REFERER, OPENAI_ENDPOINT)
        .header(ORIGIN, OPENAI_ENDPOINT)
        .header(CONTENT_TYPE, "application/json")
        .header(
            "sec-ch-ua",
            "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"",
        )
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    if let Some(token) = token {
        Ok(builder.header("openai-sentinel-chat-requirements-token", token))
    } else {
        Ok(builder)
    }    
}

pub(crate) async fn alloc_session(state: StateRef) -> Result<Session, Error> {
    let start_at = std::time::Instant::now();
    let resp = build_req(OPENAI_SENTINEL_URL, &state.device_id, None, state.clone())?.send().await;
    
    log::debug!("alloc session: {} ms, proxy: {:?} -> {:?}", start_at.elapsed().as_millis(), state.proxy, resp.as_ref().map(|r| r.status()));
    
    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("alloc session fail: {} , proxy: {:?} -> {:?}", OPENAI_SENTINEL_URL, state.proxy, e);
            return Err(e.into());
        }
    };

    let data = resp.json::<ChatRequirementsResponse>().await?;
    Ok(Session {
        start_at,
        token: data.token,
        device_id:state.device_id.clone(),
    })
}
