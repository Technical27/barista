use cocoa::command::*;
use cocoa::server::ServerData;
use cocoa::server::Status;
use std::path::Path;
use std::process::{self, Child};

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

        let mut args = vec![
            "-jar".to_string(),
            jar.to_str().unwrap().to_string(),
            "nogui".to_string(),
        ];
        cfg.args.append(&mut args);
        println!("{:?}", cfg.args);

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
        .map_err(|e| match e {
            nix::Error::Sys(c) => CommandError::SystemError(c as i32),
            _ => CommandError::UnknownSystemError,
        })?;
        self.data.status = Status::Stopped;

        Ok(CommandResponse::UpdateServer(
            self.data.id,
            self.data.clone(),
        ))
    }

    #[cfg(windows)]
    pub fn stop(&mut self) -> CommandResult {
        unimplemented!();
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
