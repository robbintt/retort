use crate::hooks::Hook;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
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
        // Clean up any markdown code blocks that ended up in the commit message
        let re = Regex::new(r"(?s)```[a-zA-Z]*\n?.*?\n?```")?;
        let cleaned_commit_message = re.replace_all(&commit_message, "");

        Ok((cleaned_commit_message.trim().to_string(), changes))
    }

    fn apply_and_commit_changes(
        &self,
        commit_message: &str,
        changes: &[FileChange],
        project_root: &Option<PathBuf>,
    ) -> anyhow::Result<()> {
        if changes.is_empty() {
            return Ok(());
        }

        if let Some(root) = project_root {
            for change in changes {
                let path = PathBuf::from(&change.path);
                let absolute_path = if path.is_absolute() {
                    path.clone()
                } else {
                    std::env::current_dir()?.join(path)
                };
                let canonical_path = if absolute_path.exists() {
                    absolute_path.canonicalize()?
                } else {
                    // For a new file, canonicalize the parent and append the filename.
                    let parent = absolute_path.parent().ok_or_else(|| {
                        anyhow::anyhow!(
                            "Could not get parent directory for {}",
                            absolute_path.display()
                        )
                    })?;
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                    let canonical_parent = parent.canonicalize()?;
                    let file_name = absolute_path.file_name().ok_or_else(|| {
                        anyhow::anyhow!("Could not get file name for {}", absolute_path.display())
                    })?;
                    canonical_parent.join(file_name)
                };
                if !canonical_path.starts_with(root) {
                    anyhow::bail!(
                        "Attempted to modify file {} which is outside the project root {}.",
                        change.path,
                        root.display()
                    );
                }
            }
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

            if let Some(parent) = Path::new(&change.path).parent() {
                fs::create_dir_all(parent)?;
            }

            let mut final_content = new_content;
            if !final_content.is_empty() && !final_content.ends_with('\n') {
                final_content.push('\n');
            }
            fs::write(&change.path, final_content)?;
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
    fn post_send(&self, llm_response: &str, project_root: &Option<PathBuf>) -> anyhow::Result<()> {
        let (commit_message, changes) = self.parse_changes(llm_response)?;
        if !changes.is_empty() {
            self.apply_and_commit_changes(&commit_message, &changes, project_root)?;
        }
        Ok(())
    }
}
