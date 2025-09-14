use crate::db::HistoryMessage;
use anyhow::Result;
use minijinja::Environment;
use serde::Serialize;

// Stubbed data from Python _build_diff_fenced_context
const REPO_MAP: &str = "";
const READ_ONLY_FILES: &str = "";
const CHAT_FILES: &str = "";
const REPO_MAP_PREFIX: &str = "The user has provided a map of the repo.";
const READ_ONLY_FILES_PREFIX: &str = "The user has provided the following read-only files:";
const CHAT_FILES_PREFIX: &str =
    "The user has added these files to the chat. You may propose edits to them.";
const RENAME_WITH_SHELL: &str =
    "To rename files which have been added to the chat, use shell commands at the end of your response.";
const GO_AHEAD_TIP: &str = "If the user just says something like \"ok\" or \"go ahead\" or \"do that\" they probably want you to make SEARCH/REPLACE blocks for the code changes you just proposed.\nThe user will say when they've applied your edits. If they haven't explicitly confirmed the edits have been applied, they probably want proper SEARCH/REPLACE blocks.";
const LAZY_PROMPT: &str = "";
const OVEREAGER_PROMPT: &str = "Pay careful attention to the scope of the user's request.\nDo what they ask, but no more.\nDo not improve, comment, fix or modify unrelated parts of the code in any way!";
const SYSTEM_REMINDER: Option<&str> = None;
const USER_LANGUAGE: Option<&str> = None;

#[derive(Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct DiffFencedContext {
    repo_map: &'static str,
    read_only_files: &'static str,
    chat_files: &'static str,
    done_messages: Vec<Message>,
    cur_messages: Vec<Message>,
    fence: &'static str,
    platform: String,
    language: &'static str,
    include_shell_commands: bool,
    example_messages: Vec<Message>,
    repo_map_prefix: &'static str,
    read_only_files_prefix: &'static str,
    chat_files_prefix: &'static str,
    rename_with_shell: &'static str,
    go_ahead_tip: &'static str,
    use_quad_backticks: bool,
    lazy_prompt: &'static str,
    overeager_prompt: &'static str,
    user_language: Option<&'static str>,
    system_reminder: Option<&'static str>,
}

pub fn build_prompt(
    done_messages: Vec<HistoryMessage>,
    cur_messages: Vec<HistoryMessage>,
) -> Result<String> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("prompts/"));

    let fence = "```";

    let platform_info = format!(
        "- Platform: {}-{}\n- Shell: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string())
    );

    let context = DiffFencedContext {
        repo_map: REPO_MAP,
        read_only_files: READ_ONLY_FILES,
        chat_files: CHAT_FILES,
        done_messages: done_messages
            .into_iter()
            .map(|m| Message {
                role: m.role,
                content: m.content,
            })
            .collect(),
        cur_messages: cur_messages
            .into_iter()
            .map(|m| Message {
                role: m.role,
                content: m.content,
            })
            .collect(),
        fence,
        platform: platform_info,
        language: "the same language they are using",
        include_shell_commands: true,
        example_messages: vec![],
        repo_map_prefix: REPO_MAP_PREFIX,
        read_only_files_prefix: READ_ONLY_FILES_PREFIX,
        chat_files_prefix: CHAT_FILES_PREFIX,
        rename_with_shell: RENAME_WITH_SHELL,
        go_ahead_tip: GO_AHEAD_TIP,
        use_quad_backticks: fence == "````",
        lazy_prompt: LAZY_PROMPT,
        overeager_prompt: OVEREAGER_PROMPT,
        user_language: USER_LANGUAGE,
        system_reminder: SYSTEM_REMINDER,
    };

    let tmpl = env.get_template("diff_fenced.j2")?;
    let res = tmpl.render(&context)?;
    Ok(res)
}

pub fn split_chat_history_markdown(text: &str) -> Vec<Message> {
    let mut messages = Vec::new();
    let lines = text.lines();
    let mut current_message_lines: Vec<&str> = Vec::new();
    let mut current_role: Option<String> = None;

    for line in lines {
        if let Some(stripped) = line.strip_prefix("## ") {
            let role_candidate = stripped.trim().to_lowercase();
            if ["system", "user", "assistant"].contains(&role_candidate.as_str()) {
                // Found a new role, so commit the previous message
                if let Some(role) = current_role.take() {
                    let content = current_message_lines.join("\n").trim().to_string();
                    // Don't add empty messages
                    if !content.is_empty() {
                        messages.push(Message { role, content });
                    }
                }
                current_message_lines.clear();
                current_role = Some(role_candidate);
            } else {
                // Not a valid role, so it's content
                current_message_lines.push(line);
            }
        } else {
            // It's a content line
            current_message_lines.push(line);
        }
    }

    // commit the very last message
    if let Some(role) = current_role.take() {
        let content = current_message_lines.join("\n").trim().to_string();
        if !content.is_empty() {
            messages.push(Message { role, content });
        }
    }

    messages
}
