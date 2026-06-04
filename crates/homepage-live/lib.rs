use std::{os::unix::process::CommandExt, path::PathBuf, process::Command};

pub trait LiveTemplate {
    fn path_to_monitor(&self) -> PathBuf;
}

pub fn restart() -> ! {
    let err = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(std::env::args())
        .exec();
    panic!("{err:?}")
}

