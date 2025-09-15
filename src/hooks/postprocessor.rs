use crate::hooks::Hook;
use regex::Regex;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct FileChange {
    pub path: String,
    pub diff_content: String,
}

pub struct PostprocessorHook {}

impl PostprocessorHook {
    fn parse_changes(&self, response: &str) -> anyhow::Result<(String, Vec<FileChange>)> {
        // This regex captures a file path followed by a fenced diff block.
        let re = Regex::new(r"(?m)^([\w\./\-_]+)\n```diff\n([\s\S]*?)```$")?;
        let mut changes = Vec::new();
        let mut last_end = 0;
        let mut commit_message_parts = Vec::new();

        for cap in re.captures_iter(response) {
            let path = cap.get(1).unwrap().as_str().trim().to_string();
            let diff_content = cap.get(2).unwrap().as_str().to_string();

            let change = FileChange { path, diff_content };
            changes.push(change);

            let full_match = cap.get(0).unwrap();
            commit_message_parts.push(response[last_end..full_match.start()].to_string());
            last_end = full_match.end();
        }
        commit_message_parts.push(response[last_end..].to_string());

        let commit_message = commit_message_parts.join("").trim().to_string();

        Ok((commit_message, changes))
    }

    fn apply_and_commit_changes(
        &self,
        commit_message: &str,
        changes: &[FileChange],
    ) -> anyhow::Result<()> {
        if changes.is_empty() {
            return Ok(());
        }

        for change in changes {
            println!("Applying patch for {}", change.path);
            let mut child = Command::new("patch")
                .arg("-p1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(change.diff_content.as_bytes())?;
            }

            let output = child.wait_with_output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("patch command failed for {}:\n{}", change.path, stderr);
            }
        }

        println!("Staging changes...");
        for change in changes {
            let status = Command::new("git").arg("add").arg(&change.path).status()?;
            if !status.success() {
                anyhow::bail!("git add failed for {}", change.path);
            }
        }

        let final_commit_message = if commit_message.is_empty() {
            "Apply changes from LLM".to_string()
        } else {
            commit_message.to_string()
        };

        println!("Committing changes with message: {}", final_commit_message);
        let status = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(&final_commit_message)
            .status()?;

        if !status.success() {
            anyhow::bail!("git commit failed");
        }

        println!("Changes committed successfully.");

        Ok(())
    }
}

impl Hook for PostprocessorHook {
    fn post_send(&self, llm_response: &str) -> anyhow::Result<()> {
        let (commit_message, changes) = self.parse_changes(llm_response)?;
        if !changes.is_empty() {
            self.apply_and_commit_changes(&commit_message, &changes)?;
        }
        Ok(())
    }
}
