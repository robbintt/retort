pub mod postprocessor;

use std::path::PathBuf;

pub trait Hook {
    fn post_send(&self, llm_response: &str, project_root: &Option<PathBuf>) -> anyhow::Result<()>;
}

pub struct HookManager {
    hooks: Vec<Box<dyn Hook>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    pub fn run_post_send_hooks(
        &self,
        llm_response: &str,
        project_root: &Option<PathBuf>,
    ) -> anyhow::Result<()> {
        for hook in &self.hooks {
            hook.post_send(llm_response, project_root)?;
        }
        Ok(())
    }
}
