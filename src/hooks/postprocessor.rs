use crate::hooks::Hook;
use regex::Regex;
use std::fs;
use std::process::Command;

#[derive(Debug)]
pub struct FileChange {
    pub path: String,
    pub search_content: String,
    pub replace_content: String,
}

pub struct PostprocessorHook {}

impl PostprocessorHook {
    fn parse_changes(&self, response: &str) -> anyhow::Result<(String, Vec<FileChange>)> {
        // This regex captures a file path followed by a SEARCH/REPLACE block.
        let re = Regex::new(
            r"(?ms)^([\w\./\-_]+)\n<<<<<<< SEARCH\n(.*?)\n=======\n(.*?)\n>>>>>>> REPLACE$",
        )?;
        let mut changes = Vec::new();
        let mut last_end = 0;
        let mut commit_message_parts = Vec::new();

        for cap in re.captures_iter(response) {
            let path = cap.get(1).unwrap().as_str().trim().to_string();
            let search_content = cap.get(2).unwrap().as_str().to_string();
            let replace_content = cap.get(3).unwrap().as_str().to_string();

            let change = FileChange {
                path,
                search_content,
                replace_content,
            };
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
            println!("Applying changes to {}", change.path);

            let new_content = if change.search_content.is_empty() {
                // An empty search block means replace the entire file.
                change.replace_content.clone()
            } else {
                // A non-empty search block means find and replace a specific part of the file.
                let original_content = fs::read_to_string(&change.path)?;
                let occurrences = original_content.matches(&change.search_content).count();

                if occurrences == 0 {
                    anyhow::bail!("SEARCH block not found in file {}", &change.path);
                }
                if occurrences > 1 {
                    anyhow::bail!(
                        "SEARCH block appears {} times in file {}. Ambiguous which one to replace.",
                        occurrences,
                        &change.path
                    );
                }

                original_content.replacen(&change.search_content, &change.replace_content, 1)
            };

            fs::write(&change.path, new_content)?;
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
