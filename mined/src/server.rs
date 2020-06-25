use minelib::command::*;
use minelib::server::ServerData;
use minelib::server::Status;
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
        let cfg = self.data.config.clone();
        let dir = Path::new(&cfg.dir);
        let jar = dir.join(cfg.jar);

        let mut args = vec![
            "-jar".to_string(),
            jar.to_str().unwrap().to_string(),
            "nogui".to_string(),
        ];
        args.append(&mut cfg.args.clone());

        process::Command::new("java")
            .args(args)
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
    pub fn stop(&mut self) {
        // use nix::sys::signal::{self, Signal};
        // use nix::unistd::Pid;
        // signal::kill(Pid::from_raw(self.process.id() as i32), Signal::SIGTERM);
        // unimplemented!();

        // fix this
        self.process.take().unwrap().kill().unwrap();
    }

    #[cfg(windows)]
    pub fn stop(&mut self) {
        unimplemented!();
    }
}
