use crate::client::build_client;
use anyhow::Result;
use bh_agent_common::{
    AgentError, BhAgentServiceClient, EnvironmentId, FileId, FileOpenMode, FileOpenType,
    ProcessChannel, ProcessId, Redirection, RemotePOpenConfig,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::{pyclass, pymethods, pymodule, PyResult, Python};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tarpc::client::RpcError;
use tarpc::context;
use tokio::runtime;

#[pyclass]
struct BhAgentClient {
    tokio_runtime: runtime::Runtime,
    client: BhAgentServiceClient,
}

fn run_in_runtime<F, R>(client: &BhAgentClient, fut: F) -> PyResult<R>
where
    F: Future<Output = Result<Result<R, AgentError>, RpcError>> + Sized,
{
    client
        .tokio_runtime
        .block_on(fut)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        .map(|r| r.map_err(|e| PyRuntimeError::new_err(e.to_string())))
        .and_then(|r| r)
}

#[pymethods]
impl BhAgentClient {
    #[staticmethod]
    fn initialize_client(ip_addr: String, port: u16) -> PyResult<Self> {
        let ip_addr = IpAddr::from_str(&ip_addr)?;
        let socket_addr = SocketAddr::new(ip_addr, port);

        let tokio_runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        match tokio_runtime.block_on(build_client(socket_addr)) {
            Ok(client) => Ok(Self {
                tokio_runtime,
                client,
            }),
            Err(e) => Err(PyRuntimeError::new_err(format!(
                "Failed to initialize client: {}",
                e
            ))),
        }
    }

    fn get_environments(&self) -> PyResult<Vec<EnvironmentId>> {
        self.tokio_runtime
            .block_on(self.client.get_environments(context::current()))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn get_tempdir(&self, env_id: EnvironmentId) -> PyResult<String> {
        run_in_runtime(self, self.client.get_tempdir(context::current(), env_id))
    }

    fn run_process(
        &self,
        env_id: EnvironmentId,
        argv: Vec<String>,
        stdin: bool,
        stdout: bool,
        stderr: bool,
        executable: Option<String>,
        env: Option<Vec<(String, String)>>,
        cwd: Option<String>,
        setuid: Option<u32>,
        setgid: Option<u32>,
        setpgid: bool,
    ) -> PyResult<ProcessId> {
        let config = RemotePOpenConfig {
            argv,
            stdin: match stdin {
                true => Redirection::Save,
                false => Redirection::None,
            },
            stdout: match stdout {
                true => Redirection::Save,
                false => Redirection::None,
            },
            stderr: match stderr {
                true => Redirection::Save,
                false => Redirection::None,
            },
            executable,
            env,
            cwd,
            setuid,
            setgid,
            setpgid,
        };
        run_in_runtime(
            self,
            self.client.run_command(context::current(), env_id, config),
        )
    }

    fn get_process_channel(
        &self,
        env_id: EnvironmentId,
        proc_id: ProcessId,
        channel: i32, // TODO: This is just 0, 1, 2 for now
    ) -> PyResult<FileId> {
        run_in_runtime(
            self,
            self.client.get_process_channel(
                context::current(),
                env_id,
                proc_id,
                match channel {
                    0 => ProcessChannel::Stdin,
                    1 => ProcessChannel::Stdout,
                    2 => ProcessChannel::Stderr,
                    _ => return Err(PyRuntimeError::new_err("Invalid channel")),
                },
            ),
        )
    }

    // File IO
    fn file_open(
        &self,
        env_id: EnvironmentId,
        path: String,
        mode_and_type: String,
    ) -> PyResult<FileId> {
        // Mode parsing
        let mut mode = FileOpenMode::Read;
        mode_and_type.chars().for_each(|c| match c {
            'r' => mode = FileOpenMode::Read,
            'w' => mode = FileOpenMode::Write,
            'x' => mode = FileOpenMode::ExclusiveWrite,
            'a' => mode = FileOpenMode::Append,
            '+' => mode = FileOpenMode::Update,
            _ => {}
        });

        // Type parsing
        let mut type_ = FileOpenType::Text;
        if mode_and_type.contains("b") {
            type_ = FileOpenType::Binary;
        }

        run_in_runtime(
            self,
            self.client
                .file_open(context::current(), env_id, path, mode, type_),
        )
    }

    fn file_close(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<()> {
        run_in_runtime(self, self.client.file_close(context::current(), env_id, fd))
    }

    fn file_is_closed(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<bool> {
        run_in_runtime(
            self,
            self.client.file_is_closed(context::current(), env_id, fd),
        )
    }

    fn file_is_readable(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<bool> {
        run_in_runtime(
            self,
            self.client.file_is_readable(context::current(), env_id, fd),
        )
    }

    fn file_read(&self, env_id: EnvironmentId, fd: FileId, num_bytes: u32) -> PyResult<Vec<u8>> {
        run_in_runtime(
            self,
            self.client
                .file_read(context::current(), env_id, fd, num_bytes),
        )
    }

    fn file_read_lines(
        &self,
        env_id: EnvironmentId,
        fd: FileId,
        hint: u32,
    ) -> PyResult<Vec<Vec<u8>>> {
        run_in_runtime(
            self,
            self.client
                .file_read_lines(context::current(), env_id, fd, hint),
        )
    }

    fn file_is_seekable(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<bool> {
        run_in_runtime(
            self,
            self.client.file_is_seekable(context::current(), env_id, fd),
        )
    }

    fn file_seek(
        &self,
        env_id: EnvironmentId,
        fd: FileId,
        offset: i32,
        whence: i32,
    ) -> PyResult<()> {
        run_in_runtime(
            self,
            self.client
                .file_seek(context::current(), env_id, fd, offset, whence),
        )
    }

    fn file_tell(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<i32> {
        run_in_runtime(self, self.client.file_tell(context::current(), env_id, fd))
    }

    fn file_is_writable(&self, env_id: EnvironmentId, fd: FileId) -> PyResult<bool> {
        run_in_runtime(
            self,
            self.client.file_is_writable(context::current(), env_id, fd),
        )
    }

    fn file_write(&self, env_id: EnvironmentId, fd: FileId, data: Vec<u8>) -> PyResult<()> {
        run_in_runtime(
            self,
            self.client.file_write(context::current(), env_id, fd, data),
        )
    }
}

#[pymodule]
pub fn bh_agent_client(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<BhAgentClient>()?;
    Ok(())
}
