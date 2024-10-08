use super::application::RuntimeApplication;
use crate::{
    entry_info,
    file_watch::{FileChangedFunc, FileWatch},
};
use notify::ReadDirectoryChangesWatcher;
use notify_debouncer_mini::DebouncedEvent;
use notify_debouncer_mini::Debouncer;
use rs_core_minimal::{file_manager, path_ext::CanonicalizeSlashExt};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
};

struct ScriptWatcher {
    receiver: Receiver<std::result::Result<Vec<DebouncedEvent>, notify::Error>>,
    _debouncer: Debouncer<ReadDirectoryChangesWatcher>,
    watch_folder_path: PathBuf,
    watch_file_name: String,
}
pub struct DotnetRuntime {
    pub application: RuntimeApplication,
    file_watch: FileWatch,
    script_watcher: Option<ScriptWatcher>,
    index: usize,
}

impl DotnetRuntime {
    pub fn default() -> crate::error::Result<DotnetRuntime> {
        let path = file_manager::get_engine_root_dir()
            .join("ExampleApplication/ExampleApplication/bin/Debug");
        Self::from_host_name("ExampleApplication".to_string(), path)
    }

    pub fn from_host_name(
        name: String,
        path: impl AsRef<Path>,
    ) -> crate::error::Result<DotnetRuntime> {
        let root_dir = path.as_ref();
        let config_path = root_dir
            .join(format!("{}.runtimeconfig.json", &name))
            .to_str()
            .ok_or(crate::error::Error::Other("".to_string()))?
            .to_string();
        let assembly_path = root_dir
            .join(format!("{}.dll", &name))
            .to_str()
            .ok_or(crate::error::Error::Other("".to_string()))?
            .to_string();
        let type_name = format!("{}.Entry, {}", &name, &name);
        let method_name = "Main".to_string();
        Self::new(config_path, assembly_path, type_name, method_name)
    }

    fn new(
        config_path: String,
        assembly_path: String,
        type_name: String,
        method_name: String,
    ) -> crate::error::Result<DotnetRuntime> {
        let application: RuntimeApplication;
        let mut file_watch = FileWatch {
            file_changed_func: std::ptr::null_mut(),
        };

        unsafe {
            type EntryPointFn = unsafe extern "C" fn(entry_info: *mut std::ffi::c_void);
            let mut entry_info = entry_info::EntryInfo::new(&mut file_watch);

            let entry_point_func: *mut EntryPointFn =
                rs_dotnet::dotnet::load_and_get_entry_point_func(
                    config_path,
                    assembly_path,
                    type_name,
                    method_name,
                )
                .map_err(|err| crate::error::Error::Dotnet(err))?;

            let entry_point_func: EntryPointFn = std::mem::transmute(entry_point_func);
            entry_point_func((&mut entry_info) as *mut _ as *mut std::ffi::c_void);
            application = RuntimeApplication::new(entry_info.runtime_application);
        }
        Ok(DotnetRuntime {
            application,
            file_watch,
            script_watcher: None,
            index: 0,
        })
    }

    pub fn reload_script(&mut self) -> crate::error::Result<()> {
        let script_watcher = self
            .script_watcher
            .as_mut()
            .ok_or(crate::error::Error::Other(format!("Did not start watch")))?;
        let path = script_watcher
            .watch_folder_path
            .join(&script_watcher.watch_file_name);
        self.reload_script_internal(path)
    }

    fn reload_script_internal(&mut self, path: impl AsRef<Path>) -> crate::error::Result<()> {
        let path = path
            .as_ref()
            .canonicalize_slash()
            .map_err(|err| crate::error::Error::IO(err, None))?;

        let extension = path
            .extension()
            .ok_or(crate::error::Error::Other("No extension".to_string()))?
            .to_string_lossy()
            .to_string();
        let stem = path
            .file_stem()
            .ok_or(crate::error::Error::Other("No stem".to_string()))?
            .to_string_lossy()
            .to_string();
        let new_file_name = format!("{}_{}.{}", stem, self.index, extension);
        let parent = path
            .parent()
            .ok_or(crate::error::Error::Other("No parent".to_string()))?;
        let source = path.clone();
        let target = parent.join(new_file_name);
        std::fs::copy(&source, &target).map_err(|err| crate::error::Error::IO(err, None))?;

        let mut path = target
            .canonicalize_slash()
            .map_err(|err| crate::error::Error::IO(err, None))?
            .to_str()
            .ok_or(crate::error::Error::Other(format!("Convert failed")))?
            .to_string();

        path.push('\0');
        let binding = unsafe { std::ffi::CStr::from_ptr(path.as_ptr() as _) };
        let c_str = binding.as_ptr();

        unsafe {
            let file_changed_func: FileChangedFunc =
                std::mem::transmute(self.file_watch.file_changed_func);
            file_changed_func(c_str);
        }
        self.index += 1;
        Ok(())
    }

    pub fn start_watch(
        &mut self,
        folder: impl AsRef<Path>,
        file_name: &str,
    ) -> crate::error::Result<()> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut debouncer =
            notify_debouncer_mini::new_debouncer(std::time::Duration::from_millis(1000), sender)
                .map_err(|err| crate::error::Error::Debouncer(err))?;

        debouncer
            .watcher()
            .watch(folder.as_ref(), notify::RecursiveMode::Recursive)
            .map_err(|err| crate::error::Error::Debouncer(err))?;
        let script_watcher = ScriptWatcher {
            receiver,
            _debouncer: debouncer,
            watch_folder_path: folder.as_ref().to_path_buf(),
            watch_file_name: file_name.to_string(),
        };
        self.script_watcher = Some(script_watcher);
        Ok(())
    }

    pub fn is_need_reload(&self) -> bool {
        if let Some(watcher) = self.script_watcher.as_ref() {
            let mut is_need_reload = false;
            for events in watcher.receiver.try_iter().filter(|x| x.is_ok()).flatten() {
                for debounced_event in events {
                    if debounced_event
                        .path
                        .file_name()
                        .map(|x| x.to_str())
                        .unwrap_or_default()
                        .unwrap_or_default()
                        == watcher.watch_file_name
                    {
                        is_need_reload = true;
                        break;
                    }
                }
            }
            return is_need_reload;
        } else {
            return false;
        }
    }

    pub fn is_watching(&self) -> bool {
        self.script_watcher.is_some()
    }
}
