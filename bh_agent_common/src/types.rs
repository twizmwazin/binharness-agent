use serde::{Deserialize, Serialize};

pub type EnvironmentId = u64;
pub type ProcessId = u64;
pub type FileId = u64;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ProcessChannel {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum Redirection {
    #[default]
    None,
    Save,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RemotePOpenConfig {
    pub argv: Vec<String>,
    pub stdin: Redirection,
    pub stdout: Redirection,
    pub stderr: Redirection,
    pub executable: Option<String>,
    pub env: Option<Vec<(String, String)>>,
    pub cwd: Option<String>,
    pub setuid: Option<u32>,
    pub setgid: Option<u32>,
    pub setpgid: bool,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FileOpenMode {
    Read,
    Write,
    ExclusiveWrite,
    Append,
    Update,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FileOpenType {
    Binary,
    Text,
}
