//! Tools module for Requiem Agent
//! 
//! This module provides all the tools available to the agent,
//! including search, parsing, diffing, version control, and more.

pub mod search;
pub mod parser;
pub mod diff;
pub mod vcs;
pub mod file_finder;
pub mod workspace;
pub use workspace::{workspace_tools_schema, execute_workspace_tool};

pub use search::SearchTool;
pub use parser::ParserTool;
pub use diff::DiffTool;
pub use vcs::VcsTool;
pub use file_finder::FileFinderTool;

/// Configuration for all tools
#[derive(Debug, Clone)]
pub struct ToolsConfig {
    pub search: search::SearchConfig,
    pub parser: parser::ParserConfig,
    pub diff: diff::DiffConfig,
    pub vcs: vcs::VcsConfig,
    pub file_finder: file_finder::FileFinderConfig,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            search: search::SearchConfig::default(),
            parser: parser::ParserConfig::default(),
            diff: diff::DiffConfig::default(),
            vcs: vcs::VcsConfig::default(),
            file_finder: file_finder::FileFinderConfig::default(),
        }
    }
}

/// Collection of all tools available to the agent
pub struct AgentTools {
    pub search: SearchTool,
    pub parser: ParserTool,
    pub diff: DiffTool,
    pub vcs: VcsTool,
    pub file_finder: FileFinderTool,
}

impl AgentTools {
    /// Create a new collection of tools with default configuration
    pub fn default() -> Self {
        Self::new(ToolsConfig::default())
    }
    
    /// Create a new collection of tools with given configuration
    pub fn new(config: ToolsConfig) -> Self {
        Self {
            search: SearchTool::new(config.search),
            parser: ParserTool::new(config.parser),
            diff: DiffTool::new(config.diff),
            vcs: VcsTool::new(config.vcs),
            file_finder: FileFinderTool::new(config.file_finder),
        }
    }
}
