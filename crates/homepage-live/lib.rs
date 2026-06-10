pub use askama;
pub use homepage_macros::LiveTemplate;

use askama::Template;

pub trait LiveTemplate: Template {
    fn render_live(&self) -> Result<String, askama::Error>;
}

#[cfg(feature = "live")]
pub use live::*;
#[cfg(feature = "live")]
mod live {
    pub use inventory;

    use dlopen::raw::Library;
    use notify::{Config, Event, EventKind, RecursiveMode, Watcher};
    use std::convert::Infallible;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;
    use std::{
        io,
        os::unix::process::CommandExt,
        process::Command,
        sync::{Mutex, mpsc},
    };
    use tracing::{debug, error, info};

    #[derive(Debug)]
    pub struct TemplateMetadata {
        pub path: &'static str,
        pub do_render: extern "Rust" fn(template: *const ()) -> Result<String, askama::Error>,
    }

    inventory::collect!(TemplateMetadata);

    #[unsafe(no_mangle)]
    pub extern "Rust" fn get_templates() -> Vec<&'static TemplateMetadata> {
        inventory::iter::<TemplateMetadata>.into_iter().collect()
    }

    pub static CURRENT_TEMPLATES: Mutex<(Option<Library>, Vec<&'static TemplateMetadata>)> =
        Mutex::new((None, Vec::new()));

    pub fn restart() -> io::Result<Infallible> {
        let mut child = Command::new("cargo")
            .arg("build")
            .args(["--features", "live"])
            .envs(std::env::vars())
            .spawn()?;
        child.wait()?;

        let err = Command::new("cargo")
            .arg("run")
            .args(["--features", "live"])
            .arg("--")
            .args(std::env::args().into_iter().skip(1))
            .envs(std::env::vars())
            .exec();
        panic!("{err:?}")
    }

    pub fn reload() -> io::Result<()> {
        // build the cdylib version
        let mut child = Command::new("cargo")
            .arg("build")
            .args(["-p", "homepage-lib"])
            .spawn()?;
        child.wait()?;

        let mut current_templates = CURRENT_TEMPLATES.lock().unwrap();
        let library = Library::open(Path::new("target/debug/libhomepage.so")).expect("open dylib");
        let sym: extern "Rust" fn() -> Vec<&'static TemplateMetadata> =
            unsafe { library.symbol("get_templates").expect("get symbol") };
        let new_templates = sym();

        debug!("{new_templates:?}");
        if current_templates.1.is_empty() || current_templates.1.len() == new_templates.len() {
            current_templates.1 = new_templates;
        } else {
            assert_eq!(new_templates.len(), current_templates.1.len() * 2);
            current_templates.1 = new_templates[new_templates.len() / 2..].to_vec();
        }

        // unload the old library
        drop(current_templates.0.replace(library));

        Ok(())
    }

    pub fn start_watching(
        soft_reload_paths: &[&Path],
        hard_reload_paths: &[&Path],
    ) -> notify::Result<()> {
        info!("configuring watcher");
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.configure(
            Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(true),
        )?;
        debug!(
            "watching relative to {}",
            std::env::current_dir().unwrap().display()
        );
        for i in soft_reload_paths {
            debug!("watching {i:?}");
            watcher.watch(i, RecursiveMode::Recursive)?;
        }
        for i in hard_reload_paths {
            debug!("watching {i:?}");
            watcher.watch(i, RecursiveMode::Recursive)?;
        }

        let soft_reload_paths = soft_reload_paths
            .into_iter()
            .map(|i| i.to_path_buf())
            .collect::<Vec<_>>();
        let hard_reload_paths = hard_reload_paths
            .into_iter()
            .map(|i| i.canonicalize())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        thread::spawn(move || {
            // keep a handle to the watcher
            let _watcher = watcher;

            info!("started watching for file events");
            while let Ok(event) = rx.recv() {
                let event = match event {
                    Ok(i) => i,
                    Err(e) => {
                        error!("watch error: {e:?}");
                        continue;
                    }
                };

                let event_kind_matches = match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => true,
                    EventKind::Other | EventKind::Any | EventKind::Access(_) => false,
                };
                let canonical_paths = event
                    .paths
                    .iter()
                    .filter_map(|i| i.canonicalize().ok())
                    .collect::<Vec<_>>();
                let hard_reload = canonical_paths
                    .iter()
                    .any(|p| hard_reload_paths.iter().any(|r| p.starts_with(r)));

                if !canonical_paths.is_empty() && event_kind_matches {
                    // effectively debounce
                    thread::sleep(Duration::from_millis(500));
                    // discard any other events
                    while let Ok(_) = rx.try_recv() {}

                    debug!("{event:?} {hard_reload}");
                    if hard_reload {
                        let Err(e) = restart();
                        error!("failed to reload: {e}");
                    } else {
                        if let Err(e) = reload() {
                            error!("failed to reload: {e}");
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
