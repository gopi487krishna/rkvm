use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use std::fs::File;

#[derive(Clone)]
pub struct ClipSyncOptions {
    pub clip_sync_enabled: bool,
    pub sync_provider_path: String,
    pub xdg_runtime_dir: String,
    pub wayland_display: String,
    pub piknik_config_path: String,
    pub uid: u32,
    pub gid: u32,
}

pub fn run_provider(clip_sync_options: &ClipSyncOptions) {
    let file = File::create("/tmp/meow.txt").unwrap();
    let stdio = Stdio::from(file);
    println!("File Path :{}", clip_sync_options.piknik_config_path);
    Command::new(&clip_sync_options.sync_provider_path)
    .env("XDG_RUNTIME_DIR", &clip_sync_options.xdg_runtime_dir)
    .env("WAYLAND_DISPLAY", &clip_sync_options.wayland_display)
    .env("PIKNIK_CONFIG", &clip_sync_options.piknik_config_path)
    .uid(clip_sync_options.uid)
    .gid(clip_sync_options.gid)
    .stdout(stdio)
    .spawn()
    .expect("Failed");
}


