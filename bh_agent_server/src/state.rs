use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::sync::{Arc, RwLock};

use subprocess::{Popen, PopenConfig};

use bh_agent_common::AgentError::{
    InvalidFileDescriptor, InvalidProcessId, IoError, ProcessStartFailure,
};
use bh_agent_common::{
    AgentError, FileId, FileOpenMode, FileOpenType, ProcessChannel, ProcessId, Redirection,
    RemotePOpenConfig,
};

// TODO: Someday a simple in-memory key value store might be a good idea
pub struct BhAgentState {
    files: RwLock<HashMap<FileId, Arc<RwLock<File>>>>,
    file_modes: RwLock<HashMap<FileId, FileOpenMode>>,
    file_types: RwLock<HashMap<FileId, FileOpenType>>,
    processes: RwLock<HashMap<ProcessId, Arc<RwLock<Popen>>>>,
    proc_stdin_ids: RwLock<HashMap<FileId, ProcessId>>,
    proc_stdout_ids: RwLock<HashMap<FileId, ProcessId>>,
    proc_stderr_ids: RwLock<HashMap<FileId, ProcessId>>,

    next_file_id: RwLock<FileId>,
    next_process_id: RwLock<ProcessId>,
}

impl BhAgentState {
    pub fn new() -> BhAgentState {
        Self {
            files: RwLock::new(HashMap::new()),
            file_modes: RwLock::new(HashMap::new()),
            file_types: RwLock::new(HashMap::new()),
            processes: RwLock::new(HashMap::new()),
            proc_stdin_ids: RwLock::new(HashMap::new()),
            proc_stdout_ids: RwLock::new(HashMap::new()),
            proc_stderr_ids: RwLock::new(HashMap::new()),

            next_file_id: RwLock::new(0),
            next_process_id: RwLock::new(0),
        }
    }

    fn take_file_id(&self) -> Result<FileId, AgentError> {
        let mut next_file_id = self.next_file_id.write()?;
        let file_id = *next_file_id;
        *next_file_id += 1;
        Ok(file_id)
    }

    fn take_proc_id(&self) -> Result<ProcessId, AgentError> {
        let mut next_process_id = self.next_process_id.write()?;
        let process_id = *next_process_id;
        *next_process_id += 1;
        Ok(process_id)
    }

    pub fn file_has_any_mode(
        &self,
        fd: &FileId,
        modes: &Vec<FileOpenMode>,
    ) -> Result<bool, AgentError> {
        Ok(modes.contains(
            self.file_modes
                .read()?
                .get(&fd)
                .ok_or(InvalidFileDescriptor)?,
        ))
    }

    pub fn file_type(&self, fd: &FileId) -> Result<FileOpenType, AgentError> {
        Ok(self
            .file_types
            .read()?
            .get(&fd)
            .ok_or(InvalidFileDescriptor)
            .and_then(|t| Ok(t.clone()))?)
    }

    pub fn open_path(
        &self,
        path: String,
        mode: FileOpenMode,
        type_: FileOpenType,
    ) -> Result<FileId, AgentError> {
        let mut open_opts = OpenOptions::new();
        match mode {
            FileOpenMode::Read => open_opts.read(true),
            FileOpenMode::Write => open_opts.write(true).create(true),
            FileOpenMode::ExclusiveWrite => open_opts.write(true).create_new(true),
            FileOpenMode::Append => open_opts.append(true),
            FileOpenMode::Update => open_opts.read(true).write(true),
        };
        let file = open_opts.open(&path).map_err(|e| {
            eprintln!("Path: {}", path);
            eprintln!("Error opening file: {}", e);
            IoError
        })?;
        let file_id = self.take_file_id()?;
        self.files
            .write()?
            .insert(file_id, Arc::new(RwLock::new(file)));
        self.file_modes.write()?.insert(file_id, mode);
        self.file_types.write()?.insert(file_id, type_);
        Ok(file_id)
    }

    pub fn run_command(&self, config: RemotePOpenConfig) -> Result<ProcessId, AgentError> {
        let mut popenconfig = PopenConfig {
            stdin: match config.stdin {
                Redirection::None => subprocess::Redirection::None,
                Redirection::Save => subprocess::Redirection::Pipe,
            },
            stdout: match config.stdout {
                Redirection::None => subprocess::Redirection::None,
                Redirection::Save => subprocess::Redirection::Pipe,
            },
            stderr: match config.stderr {
                Redirection::None => subprocess::Redirection::None,
                Redirection::Save => subprocess::Redirection::Pipe,
            },
            detached: false,
            executable: config.executable.map(|s| s.into()),
            env: config.env.map(|v| {
                v.iter()
                    .map(|t| (t.0.clone().into(), t.1.clone().into()))
                    .collect()
            }),
            cwd: config.cwd.map(|s| s.into()),
            ..PopenConfig::default()
        };
        #[cfg(unix)]
        {
            popenconfig.setuid = config.setuid.or(popenconfig.setuid);
            popenconfig.setgid = config.setuid.or(popenconfig.setgid);
            popenconfig.setpgid = config.setuid.or(popenconfig.setpgid);
        }

        let proc = Popen::create(
            config
                .argv
                .iter()
                .map(|s| OsStr::new(s))
                .collect::<Vec<_>>()
                .as_slice(),
            popenconfig,
        )
        .map_err(|_| ProcessStartFailure)?;

        let proc_id = self.take_proc_id()?;

        // Stick the process channels into the file map
        if proc.stdin.is_some() {
            let file_id = self.take_file_id()?;
            self.proc_stdin_ids.write()?.insert(file_id, proc_id);
        }
        if proc.stdout.is_some() {
            let file_id = self.take_file_id()?;
            self.proc_stdout_ids.write()?.insert(file_id, proc_id);
        }
        if proc.stderr.is_some() {
            let file_id = self.take_file_id()?;
            self.proc_stdout_ids.write()?.insert(file_id, proc_id);
        }

        // Move the proc to the process map
        self.processes
            .write()?
            .insert(proc_id, Arc::new(RwLock::new(proc)));

        Ok(proc_id)
    }

    pub fn get_process_channel(
        &self,
        proc_id: &ProcessId,
        channel: ProcessChannel,
    ) -> Result<FileId, AgentError> {
        match channel {
            ProcessChannel::Stdin => &self.proc_stdin_ids,
            ProcessChannel::Stdout => &self.proc_stdout_ids,
            ProcessChannel::Stderr => &self.proc_stderr_ids,
        }
        .read()?
        .get(&proc_id)
        .map(|i| i.clone())
        .ok_or(InvalidProcessId)
    }

    pub fn close_file(&self, fd: &FileId) -> Result<(), AgentError> {
        Ok(drop(
            self.files
                .write()?
                .remove(&fd)
                .ok_or(InvalidFileDescriptor)?,
        ))
    }

    pub fn is_file_closed(&self, fd: &FileId) -> Result<bool, AgentError> {
        Ok(self.files.read()?.contains_key(&fd))
    }

    pub fn do_mut_operation<R: Sized>(
        &self,
        fd: &FileId,
        op: impl Fn(&mut File) -> R,
    ) -> Result<R, AgentError> {
        // Get file logic
        if let Some(file_lock) = self.files.read()?.get(fd) {
            return Ok(op(&mut *file_lock.write()?));
        }

        // If these unwraps fail, the state is bad
        if let Some(pid) = self.proc_stdin_ids.read()?.get(&fd) {
            let procs_binding = self.processes.read()?;
            let mut proc_binding = procs_binding.get(pid).unwrap().write()?;
            let file = proc_binding.stdin.as_mut().unwrap();
            return Ok(op(file));
        }
        if let Some(pid) = self.proc_stdout_ids.read()?.get(&fd) {
            let procs_binding = self.processes.read()?;
            let mut proc_binding = procs_binding.get(pid).unwrap().write()?;
            let file = proc_binding.stdout.as_mut().unwrap();
            return Ok(op(file));
        }
        if let Some(pid) = self.proc_stderr_ids.read()?.get(&fd) {
            let procs_binding = self.processes.read()?;
            let mut proc_binding = procs_binding.get(pid).unwrap().write()?;
            let file = proc_binding.stderr.as_mut().unwrap();
            return Ok(op(file));
        }

        Err(InvalidFileDescriptor)
    }
}
