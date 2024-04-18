use crate::fgpt::{CompletionRequest, Message};
use atty::Stream;
use futures::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{Read, Write};
use tokio::select;

pub async fn run_repl(state: crate::StateRef) -> Result<(), crate::fgpt::Error> {
    let help_text = r#"Type `/help` for more information.
Type `/exit` to exit the program.
Type `/reset` to reset the conversation.
    "#;

    println!("Ctrl-C to cancel, Ctrl-D to exit. '\' for a new line. ✨");

    let mut rl = DefaultEditor::new()?;
    let mut prompt_text = ">> ".to_string();
    let mut question = String::new();

    let mut last_message_id = Some(uuid::Uuid::new_v4().to_string());
    let mut conversation_id: Option<String> = None;

    loop {
        let readline = rl.readline(&prompt_text);
        match readline {
            Ok(line) => {
                let line = line.trim();
                match line {
                    "/exit" => break,
                    "/help" => {
                        println!("{}", help_text);
                        continue;
                    }
                    "/reset" => {
                        conversation_id = None;
                        last_message_id = Some(uuid::Uuid::new_v4().to_string());
                        println!("Conversation reset. ✨");
                        continue;
                    }
                    _ => {}
                }
                if line.ends_with("\\") {
                    prompt_text = ".. ".to_string();
                    question.push_str(&line[..line.len() - 1]);
                    question.push('\n');
                    continue;
                } else {
                    prompt_text = ">> ".to_string();
                    question.push_str(line);
                }
                rl.add_history_entry(&question).ok();
                question = String::new();

                let mut messages = vec![];
                messages.push(Message {
                    role: "user".to_string(),
                    content: line.to_string(),
                    content_type: "text".to_string(),
                });

                select! {
                    r = execute_plain(
                        state.clone(),
                        messages,
                        conversation_id.clone(),
                        last_message_id.clone(),
                    ) => {
                        let r = r?;
                        conversation_id = Some(r.conversation_id);
                        last_message_id = Some(r.last_message_id);
                    }
                    _ = tokio::signal::ctrl_c() => {
                        break;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

pub async fn run(state: crate::StateRef) -> Result<(), crate::fgpt::Error> {
    if state.repl || (state.qusetion.is_none() && state.input_file.is_none()) {
        return run_repl(state).await;
    }

    let mut messages = vec![];
    if state.code {
        messages.push(Message {
            role: "system".to_string(),
            content: include_str!("./role.code.prompt").to_string(),
            content_type: "text".to_string(),
        });
    }

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

    match state.input_file {
        Some(ref file) => {
            let content = std::fs::read_to_string(file)?;
            messages.push(Message {
                role: "user".to_string(),
                content,
                content_type: "text".to_string(),
            });
        }
        None => {}
    }

    if !atty::is(Stream::Stdin) {
        // it may be a pipe or a file
        let mut content = String::new();
        std::io::stdin().read_to_string(&mut content)?;
        messages.push(Message {
            role: "user".to_string(),
            content,
            content_type: "text".to_string(),
        });
    }

    messages.iter().for_each(|m| log::debug!("{:?}", m));

    let tokenizer = gpt_tokenizer::Default::new();
    let prompt_tokens: usize = messages
        .iter()
        .map(|message| tokenizer.encode(&message.content).len())
        .sum();

    let start_at = std::time::Instant::now();
    let result = execute_plain(
        state.clone(),
        messages,
        None,
        Some(uuid::Uuid::new_v4().to_string()),
    )
    .await?;

    if state.dump_stats {
        let elapsed = start_at.elapsed().as_secs_f64();
        let completion_tokens = tokenizer.encode(&result.textbuf).len();
        let total_tokens = completion_tokens + prompt_tokens;
        let throughput = completion_tokens as f64 / elapsed as f64;

        println!(
            "Total tokens: \x1b[32m{}\x1b[0m, completion tokens: \x1b[32m{}\x1b[0m, prompt tokens: \x1b[32m{}\x1b[0m, elapsed: \x1b[33m{:.1}\x1b[0m secs, throughput: \x1b[33m{:.2}\x1b[0m tps",
            total_tokens,
            completion_tokens,
            prompt_tokens,
            elapsed,
            throughput
        );
    }
    Ok(())
}

struct CompletionResult {
    textbuf: String,
    conversation_id: String,
    last_message_id: String,
}

async fn execute_plain(
    state: crate::StateRef,
    messages: Vec<Message>,
    conversion_id: Option<String>,
    parent_message_id: Option<String>,
) -> Result<CompletionResult, crate::fgpt::Error> {
    let req = CompletionRequest::new(state.clone(), messages, conversion_id, parent_message_id);
    let mut stream = req.stream(state.clone()).await?;

    let mut textbuf = String::new();
    let mut conversation_id = String::new();
    let mut last_message_id = String::new();

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
                let delta_chars = &text[textbuf.len()..];
                textbuf = text.clone();
                print!("{}", delta_chars);
                let _ = std::io::stdout().flush();
                conversation_id = message.conversation_id.clone();
                last_message_id = message.message.id.clone();
            }
            Ok(crate::fgpt::CompletionEvent::Done) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("{:?}", e);
                break;
            }
        }
    }
    Ok(CompletionResult {
        textbuf,
        conversation_id,
        last_message_id,
    })
}
