use crate::fgpt::{self, Message};
use rustyline::highlight::Highlighter;
use rustyline::{error::ReadlineError, Editor};
use rustyline::{Completer, Helper, Highlighter, Hinter, Validator};
use std::borrow::Cow;
use std::io::{IsTerminal, Read};
use tokio::io::AsyncWriteExt;
use tokio::select;

#[derive(Default)]
struct PromptHighlighter {}

impl Highlighter for PromptHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        return Cow::Owned(format!("\x1b[33m{}\x1b[0m", line));
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        return true;
    }
}

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct PromptHelper {
    #[rustyline(Highlighter)]
    highlighter: PromptHighlighter,
}

pub async fn run_repl(state: fgpt::StateRef) -> Result<(), fgpt::Error> {
    println!("free GPT-3.5 cli tools | 🪐 https://github.com/shenjinti/fgpt");
    println!("💖 To star the repository if you like \x1b[1;32mfgpt\x1b[0m!");

    let help_texts = vec![
        "",
        "Type `\x1b[1;32m/help\x1b[0m` for more information.",
        "Type `\x1b[1;32m/exit\x1b[0m` or <\x1b[1;35mCtrl-D\x1b[0m> to exit the program.",
        "Type `\x1b[1;32m/reset\x1b[0m` to reset the conversation.",
        "",
        "Ctrl-C to cancel, Ctrl-D to exit. \x1b[1;32m\\\x1b[0m for a new line. ✨",
    ];
    help_texts.iter().for_each(|text| println!("{}", text));
    let h = PromptHelper {
        highlighter: PromptHighlighter {},
    };

    let mut rl = Editor::new()?;
    rl.set_helper(Some(h));
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
                        help_texts.iter().for_each(|text| println!("{}", text));
                        continue;
                    }
                    "/reset" => {
                        conversation_id = None;
                        last_message_id = Some(uuid::Uuid::new_v4().to_string());
                        println!("Conversation reset. ✨");
                        continue;
                    }
                    "" => continue,
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
                    content_type: Some("text".to_string()),
                });

                select! {
                    r = crate::fgpt::execute_plain(
                        state.clone(),
                        messages,
                        conversation_id.clone(),
                        last_message_id.clone(),
                         |delta| async move {
                            print!("{}", delta);
                            tokio::io::stdout().flush().await.ok();
                        },
                    ) => {
                        let r = r?;
                        conversation_id = Some(r.conversation_id);
                        last_message_id = Some(r.last_message_id);
                        println!();
                    }
                    _ = tokio::signal::ctrl_c() => {
                        log::info!("Ctrl-C pressed. Exiting.");
                        break;
                    }
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
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

pub async fn run(state: fgpt::StateRef) -> Result<(), fgpt::Error> {
    if state.repl || (state.qusetion.is_none() && state.input_file.is_none()) {
        return run_repl(state).await;
    }

    let mut messages = vec![];
    if state.code {
        messages.push(Message {
            role: "system".to_string(),
            content: include_str!("./role.code.prompt").to_string(),
            content_type: Some("text".to_string()),
        });
    }

    if let Some(ref q) = state.qusetion {
        messages.push(Message {
            role: "user".to_string(),
            content: q.clone(),
            content_type: Some("text".to_string()),
        });
    }

    if let Some(ref file) = state.input_file {
        let content = std::fs::read_to_string(file)?;
        messages.push(Message {
            role: "user".to_string(),
            content,
            content_type: Some("text".to_string()),
        });
    }

    if !std::io::stdin().is_terminal() {
        // it may be a pipe or a file
        let mut content = String::new();
        std::io::stdin().read_to_string(&mut content)?;
        messages.push(Message {
            role: "user".to_string(),
            content,
            content_type: Some("text".to_string()),
        });
    }

    messages.iter().for_each(|m| log::debug!("{:?}", m));

    let tokenizer = gpt_tokenizer::Default::new();
    let prompt_tokens: usize = messages
        .iter()
        .map(|message| tokenizer.encode(&message.content).len())
        .sum();

    let start_at = std::time::Instant::now();
    let r = crate::fgpt::execute_plain(
        state.clone(),
        messages,
        None,
        Some(uuid::Uuid::new_v4().to_string()),
        |delta| async move {
            print!("{}", delta);
            tokio::io::stdout().flush().await.ok();
        },
    )
    .await?;

    println!();

    let elapsed = start_at.elapsed().as_secs_f64();
    let completion_tokens = tokenizer.encode(&r.textbuf).len();
    let total_tokens = completion_tokens + prompt_tokens;
    let throughput = completion_tokens as f64 / elapsed as f64;

    let stats_text = format!(
        "Total tokens: \x1b[32m{}\x1b[0m, completion tokens: \x1b[32m{}\x1b[0m, prompt tokens: \x1b[32m{}\x1b[0m, elapsed: \x1b[33m{:.1}\x1b[0m secs, throughput: \x1b[33m{:.2}\x1b[0m tps",
        total_tokens,
        completion_tokens,
        prompt_tokens,
        elapsed,
        throughput
    );
    if state.dump_stats {
        println!("{}", stats_text);
    } else {
        log::debug!("{}", stats_text);
    }
    Ok(())
}
