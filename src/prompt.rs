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

pub fn build_prompt_messages(
    done_messages: Vec<HistoryMessage>,
    cur_messages: Vec<HistoryMessage>,
    read_write_files: &[(String, String)],
    read_only_files: &[(String, String)],
) -> Result<Vec<Message>> {
    #[derive(Serialize)]
    struct SystemPromptContext {
        fence: &'static str,
        platform: String,
        lazy_prompt: &'static str,
        overeager_prompt: &'static str,
        rename_with_shell: &'static str,
        go_ahead_tip: &'static str,
    }

    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("prompts/"));
    let tmpl = env.get_template("_diff_fenced_system_prompt.j2")?;

    let fence = "```";
    let platform_info = format!(
        "- Platform: {}-{}\n- Shell: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string())
    );

    let context = SystemPromptContext {
        fence,
        platform: platform_info,
        lazy_prompt: LAZY_PROMPT,
        overeager_prompt: OVEREAGER_PROMPT,
        rename_with_shell: RENAME_WITH_SHELL,
        go_ahead_tip: GO_AHEAD_TIP,
    };

    let mut system_prompt_content = tmpl.render(context)?;
    if let Some(reminder) = SYSTEM_REMINDER {
        system_prompt_content.push('\n');
        system_prompt_content.push_str(reminder);
    }

    let mut result_messages = Vec::new();

    result_messages.push(Message {
        role: "system".to_string(),
        content: system_prompt_content,
    });

    if !read_only_files.is_empty() {
        let mut content = format!("{}\n", READ_ONLY_FILES_PREFIX);
        for (path, file_content) in read_only_files {
            content.push_str(&format!("{}\n```\n{}\n```\n", path, file_content));
        }
        result_messages.push(Message {
            role: "user".to_string(),
            content,
        });
        result_messages.push(Message {
            role: "assistant".to_string(),
            content: "Ok, I will use these files as references.".to_string(),
        });
    }

    if !read_write_files.is_empty() {
        let mut content = format!("{}\n", CHAT_FILES_PREFIX);
        for (path, file_content) in read_write_files {
            content.push_str(&format!("{}\n```\n{}\n```\n", path, file_content));
        }
        result_messages.push(Message {
            role: "user".to_string(),
            content,
        });
        result_messages.push(Message {
            role: "assistant".to_string(),
            content: "Ok, any changes I propose will be to those files.".to_string(),
        });
    }

    result_messages.extend(done_messages.into_iter().map(|m| Message {
        role: m.role,
        content: m.content,
    }));
    result_messages.extend(cur_messages.into_iter().map(|m| Message {
        role: m.role,
        content: m.content,
    }));

    Ok(result_messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HistoryMessage;

    #[test]
    fn test_build_prompt_messages() {
        let done_messages = vec![
            HistoryMessage {
                role: "user".to_string(),
                content: "previous user message".to_string(),
                created_at: "".to_string(),
            },
            HistoryMessage {
                role: "assistant".to_string(),
                content: "previous assistant message".to_string(),
                created_at: "".to_string(),
            },
        ];
        let cur_messages = vec![HistoryMessage {
            role: "user".to_string(),
            content: "current user message".to_string(),
            created_at: "".to_string(),
        }];

        let messages = build_prompt_messages(done_messages, cur_messages, &[], &[]).unwrap();

        assert!(!messages.is_empty());

        // Check for system prompt
        assert_eq!(messages[0].role, "system");
        assert!(messages[0]
            .content
            .contains("Act as an expert software developer."));

        // Check for done messages
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "previous user message");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[2].content, "previous assistant message");

        // Check for current message
        assert_eq!(messages[3].role, "user");
        assert_eq!(messages[3].content, "current user message");
    }

    #[test]
    fn test_build_prompt_messages_with_files() {
        let done_messages = vec![HistoryMessage {
            role: "user".to_string(),
            content: "previous user message".to_string(),
            created_at: "".to_string(),
        }];
        let cur_messages = vec![HistoryMessage {
            role: "user".to_string(),
            content: "current user message".to_string(),
            created_at: "".to_string(),
        }];
        let read_write_files = vec![("rw.txt".to_string(), "rw content".to_string())];
        let read_only_files = vec![("ro.txt".to_string(), "ro content".to_string())];

        let messages = build_prompt_messages(
            done_messages,
            cur_messages,
            &read_write_files,
            &read_only_files,
        )
        .unwrap();

        assert_eq!(messages.len(), 7);

        // 1. System prompt
        assert_eq!(messages[0].role, "system");

        // 2. Read-only files
        assert_eq!(messages[1].role, "user");
        assert!(messages[1].content.contains(READ_ONLY_FILES_PREFIX));
        assert!(messages[1].content.contains("ro.txt"));
        assert!(messages[1].content.contains("ro content"));
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(
            messages[2].content,
            "Ok, I will use these files as references."
        );

        // 3. Read-write files
        assert_eq!(messages[3].role, "user");
        assert!(messages[3].content.contains(CHAT_FILES_PREFIX));
        assert!(messages[3].content.contains("rw.txt"));
        assert!(messages[3].content.contains("rw content"));
        assert_eq!(messages[4].role, "assistant");
        assert_eq!(
            messages[4].content,
            "Ok, any changes I propose will be to those files."
        );

        // 4. History (done_messages)
        assert_eq!(messages[5].role, "user");
        assert_eq!(messages[5].content, "previous user message");

        // 5. Current message (cur_messages)
        assert_eq!(messages[6].role, "user");
        assert_eq!(messages[6].content, "current user message");
    }
}
