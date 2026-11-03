mod args;
mod ops;
mod paths;
mod root;
#[cfg(test)]
mod testutil;

use args::MemoryArgs;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};

#[derive(Debug, Clone)]
pub struct Memorija {
    root: std::path::PathBuf,
}

#[tool_router]
impl Memorija {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }

    #[tool(description = "Persistent memory files for the user and their work across sessions. \
RECALL - For non-trivial tasks, view `/memories` at the start of a session or when \
picked up mid-session, and read what's relevant before working. Consult memory before \
answering anything about the user's past work, preferences, or projects. \
CAPTURE - Continuously record durable, work-relevant facts as they surface: the user's \
role, preferences and conventions (tools, style, how they like work done); what each \
project is and how it's built; decisions made and why; people, systems and open threads. \
Recording a memory is a background action: do it without asking and never announce it in \
your reply. Record a durable fact even if it surfaces during a trivial exchange. Never \
store secrets, credentials, or sensitive personal data; skip one-off task details. \
ORGANIZE - `/memories/about/` holds who the user is (role, preferences, conventions, \
ongoing context); `/memories/projects/<name>/` holds per-project knowledge (overview, \
decisions, open threads); `/memories/sessions/YYYY-MM.md` is a brief running log of work \
done, a few lines appended per month. Update existing memories instead of duplicating them. \
MECHANICS - Paths are relative to the memory root; a leading `/memories` prefix is optional. \
Commands: `view` shows a file with 1-based line numbers (optional `view_range: [start, end]`, \
end `-1` means through the end of the file) or recursively lists a directory; `create` writes \
or overwrites a file (`file_text`); `str_replace` replaces a unique `old_str` with `new_str` \
(omit `new_str` to delete the matched text); `insert` adds `insert_text` after line \
`insert_line` (0 = top of file); `delete` removes a file or directory recursively; `rename` \
moves the target to `new_path`. Recoverable failures are reported as error results you can \
retry with corrected arguments.")]
    async fn memory(
        &self,
        Parameters(args): Parameters<MemoryArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(match ops::run(&self.root, &args) {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
        })
    }
}

#[tool_handler]
impl ServerHandler for Memorija {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = root::resolve()?;
    root::ensure(&root)?;
    let service = Memorija::new(root).serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
