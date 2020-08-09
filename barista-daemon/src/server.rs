use barista::command::*;
use barista::server::ServerData;
use barista::server::Status;
use std::path::Path;
use std::process::{self, Child};

#[cfg(windows)]
use winapi::shared::{minwindef::BOOL, windef::HWND};

#[derive(Debug)]
pub struct Server {
    pub data: ServerData,
    process: Option<Child>,
}

impl Server {
    pub fn new(data: ServerData) -> Self {
        Self {
            data,
            process: None,
        }
    }

    pub fn start(&mut self) -> CommandResult {
        let mut cfg = self.data.config.clone();
        let dir = Path::new(&cfg.dir);
        let jar = dir.join(cfg.jar);

        let mut args = vec!["-jar".to_string(), jar.to_str().unwrap().to_string()];

        #[cfg(not(windows))]
        args.append(&mut vec!["nogui".to_string()]);

        cfg.args.append(&mut args);

        process::Command::new("java")
            .args(cfg.args)
            .current_dir(dir)
            .spawn()
            .map(|c| {
                self.process = Some(c);
                self.data.status = Status::Open;
                CommandResponse::UpdateServer(self.data.id, self.data.clone())
            })
            .map_err(|e| match e.raw_os_error() {
                Some(c) => CommandError::SystemError(c),
                None => CommandError::UnknownSystemError,
            })
    }

    #[cfg(unix)]
    pub fn stop(&mut self) -> CommandResult {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        signal::kill(
            Pid::from_raw(self.process.take().unwrap().id() as i32),
            Signal::SIGTERM,
        )
        .map_err(|e| {
            self.data.status = Status::Crashed;
            match e {
                nix::Error::Sys(c) => CommandError::SystemError(c as i32),
                _ => CommandError::UnknownSystemError,
            }
        })?;
        self.data.status = Status::Stopped;

        Ok(CommandResponse::UpdateServer(
            self.data.id,
            self.data.clone(),
        ))
    }

    #[cfg(windows)]
    unsafe extern "system" fn find_window(hwnd: HWND, pid: isize) -> BOOL {
        use winapi::um::winuser::{GetWindowThreadProcessId, PostMessageA};

        let pid = *(pid as *const u32);
        let mut w_pid = 0u32;

        GetWindowThreadProcessId(hwnd, &mut w_pid);

        if w_pid == pid {
            PostMessageA(hwnd, 0x10, 0, 0);
            return 0;
        }

        1
    }

    #[cfg(windows)]
    pub fn stop(&mut self) -> CommandResult {
        use winapi::um::winuser::EnumWindows;

        let pid = self.process.take().unwrap().id();

        unsafe {
            EnumWindows(Some(Self::find_window), (&pid as *const u32) as isize);
        }

        self.data.status = Status::Stopped;

        Ok(CommandResponse::UpdateServer(
            self.data.id,
            self.data.clone(),
        ))
    }

    pub fn update_status(&mut self) -> bool {
        if self.data.status == Status::Open {
            if let Some(code) = self
                .process
                .as_mut()
                .and_then(|c| c.try_wait().ok())
                .flatten()
            {
                self.data.status = if code.success() {
                    Status::Stopped
                } else {
                    Status::Crashed
                };
                self.process.take();

                return true;
            }
        }
        false
    }
}
