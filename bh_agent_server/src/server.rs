use std::future::{ready, Ready};
use std::io::{Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use tarpc::context::Context;

use bh_agent_common::AgentError::*;
use bh_agent_common::{
    AgentError, BhAgentService, EnvironmentId, FileId, FileOpenMode, FileOpenType, ProcessChannel,
    ProcessId, RemotePOpenConfig,
};

use crate::state::BhAgentState;
use crate::util::{read_generic, read_lines};

macro_rules! check_env_id {
    ($env_id:expr) => {
        if $env_id != 0 {
            return ready(Err(AgentError::InvalidEnvironmentId));
        }
    };
}

#[derive(Clone)]
pub struct BhAgentServer {
    sockaddr: SocketAddr,
    state: Arc<BhAgentState>,
}

impl BhAgentServer {
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self {
            sockaddr: socket_addr,
            state: Arc::new(BhAgentState::new()),
        }
    }
}

#[tarpc::server]
impl BhAgentService for BhAgentServer {
    type GetEnvironmentsFut = Ready<Vec<EnvironmentId>>;
    fn get_environments(self, _: Context) -> Self::GetEnvironmentsFut {
        // Our implementation currently only supports the default environment
        ready(vec![0])
    }

    type GetTempdirFut = Ready<Result<String, AgentError>>;
    fn get_tempdir(self, _: Context, env_id: EnvironmentId) -> Self::GetTempdirFut {
        check_env_id!(env_id);

        ready(Ok("/tmp".to_string())) // TODO: make configurable
    }

    type RunCommandFut = Ready<Result<ProcessId, AgentError>>;
    fn run_command(
        self,
        _: Context,
        env_id: EnvironmentId,
        config: RemotePOpenConfig,
    ) -> Self::RunCommandFut {
        check_env_id!(env_id);

        ready(self.state.run_command(config))
    }

    type GetProcessChannelFut = Ready<Result<FileId, AgentError>>;
    fn get_process_channel(
        self,
        _: Context,
        env_id: EnvironmentId,
        proc_id: ProcessId,
        channel: ProcessChannel,
    ) -> Self::GetProcessChannelFut {
        check_env_id!(env_id);

        ready(self.state.get_process_channel(&proc_id, channel))
    }

    type FileOpenFut = Ready<Result<FileId, AgentError>>;
    fn file_open(
        self,
        _: Context,
        env_id: EnvironmentId,
        path: String,
        mode: FileOpenMode,
        type_: FileOpenType,
    ) -> Self::FileOpenFut {
        check_env_id!(env_id);

        ready(self.state.open_path(path, mode, type_))
    }

    type FileCloseFut = Ready<Result<(), AgentError>>;
    fn file_close(self, _: Context, env_id: EnvironmentId, fd: FileId) -> Self::FileCloseFut {
        check_env_id!(env_id);

        ready(self.state.close_file(&fd))
    }

    type FileIsClosedFut = Ready<Result<bool, AgentError>>;
    fn file_is_closed(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
    ) -> Self::FileIsClosedFut {
        check_env_id!(env_id);

        ready(self.state.is_file_closed(&fd))
    }

    type FileIsReadableFut = Ready<Result<bool, AgentError>>;
    fn file_is_readable(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
    ) -> Self::FileIsReadableFut {
        check_env_id!(env_id);

        ready(
            self.state
                .file_has_any_mode(&fd, &vec![FileOpenMode::Read, FileOpenMode::Update]),
        )
    }

    type FileReadFut = Ready<Result<Vec<u8>, AgentError>>;
    fn file_read(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
        num_bytes: u32,
    ) -> Self::FileReadFut {
        check_env_id!(env_id);

        ready(
            self.state
                .do_mut_operation(&fd, |file| {
                    read_generic(file, num_bytes, self.state.file_type(&fd)?)
                })
                .and_then(|v| v.map_err(|_| IoError)),
        )
    }

    type FileReadLinesFut = Ready<Result<Vec<Vec<u8>>, AgentError>>;
    fn file_read_lines(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
        hint: u32,
    ) -> Self::FileReadLinesFut {
        check_env_id!(env_id);

        // TODO: support hint
        ready(
            self.state
                .do_mut_operation(&fd, |file| read_lines(file).map_err(|_| IoError))
                .and_then(|r| r),
        )
    }

    type FileIsSeekableFut = Ready<Result<bool, AgentError>>;
    fn file_is_seekable(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
    ) -> Self::FileIsSeekableFut {
        check_env_id!(env_id);

        todo!()
    }

    type FileSeekFut = Ready<Result<(), AgentError>>;
    fn file_seek(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
        offset: i32,
        whence: i32,
    ) -> Self::FileSeekFut {
        check_env_id!(env_id);

        let from = match whence {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset as i64),
            2 => SeekFrom::End(offset as i64),
            _ => return ready(Err(AgentError::InvalidSeekWhence)),
        };

        ready(
            self.state
                .do_mut_operation(&fd, |file| file.seek(from))
                .map(|_| ()),
        )
    }

    type FileTellFut = Ready<Result<i32, AgentError>>;
    fn file_tell(self, _: Context, env_id: EnvironmentId, fd: FileId) -> Self::FileTellFut {
        check_env_id!(env_id);

        todo!()
    }

    type FileIsWritableFut = Ready<Result<bool, AgentError>>;
    fn file_is_writable(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
    ) -> Self::FileIsWritableFut {
        check_env_id!(env_id);

        ready(self.state.file_has_any_mode(
            &fd,
            &vec![
                FileOpenMode::Write,
                FileOpenMode::ExclusiveWrite,
                FileOpenMode::Update,
                FileOpenMode::Append,
            ],
        ))
    }

    type FileWriteFut = Ready<Result<(), AgentError>>;
    fn file_write(
        self,
        _: Context,
        env_id: EnvironmentId,
        fd: FileId,
        data: Vec<u8>,
    ) -> Self::FileWriteFut {
        check_env_id!(env_id);

        ready(
            self.state
                .do_mut_operation(&fd, |file| file.write(&data))
                .map(|_| ()),
        )
    }
}
