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
        let lines: Vec<&str> = response.lines().collect();
        let mut changes = Vec::new();
        let mut block_line_indices = std::collections::HashSet::new();

        for i in 0..lines.len() {
            // A block starts with a file path on one line, and "<<<<<<< SEARCH" on the next
            if lines.get(i + 1) == Some(&"<<<<<<< SEARCH") {
                let path = lines[i].trim();
                // Basic heuristic to ensure the path looks like a path
                if path.is_empty() || path.contains(' ') || path.starts_with('#') {
                    continue;
                }

                let mut search_content_lines = Vec::new();
                let mut replace_content_lines = Vec::new();
                let mut in_search_section = true;
                let mut block_found = false;

                // Start searching from after the "<<<<<<< SEARCH" line
                let mut j = i + 2;
                while j < lines.len() {
                    if lines[j] == "=======" {
                        in_search_section = false;
                    } else if lines[j] == ">>>>>>> REPLACE" {
                        block_found = true;
                        break;
                    } else if in_search_section {
                        search_content_lines.push(lines[j]);
                    } else {
                        replace_content_lines.push(lines[j]);
                    }
                    j += 1;
                }

                if block_found {
                    // Mark all lines from the path to the end of the block for exclusion from the commit message
                    for k in i..=j {
                        block_line_indices.insert(k);
                    }
                    changes.push(FileChange {
                        path: path.to_string(),
                        search_content: search_content_lines.join("\n"),
                        replace_content: replace_content_lines.join("\n"),
                    });
                }
            }
        }

        let mut commit_message_parts = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if !block_line_indices.contains(&i) {
                commit_message_parts.push(*line);
            }
        }

        let commit_message = commit_message_parts.join("\n");
        // Clean up any markdown fences that ended up in the commit message
        let re = Regex::new(r"```[a-zA-Z]*|```")?;
        let cleaned_commit_message = re.replace_all(&commit_message, "");

        Ok((cleaned_commit_message.trim().to_string(), changes))
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
