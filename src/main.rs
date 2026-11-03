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

    #[tool(description = "Interact with persistent memory files that survive between sessions. \
Paths are relative to the memory root (`/memories`); a leading `/memories` prefix is optional. \
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
