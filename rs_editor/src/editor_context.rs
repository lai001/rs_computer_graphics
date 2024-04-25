use crate::{
    build_config::EBuildType,
    content_folder::ContentFolder,
    custom_event::{ECustomEventType, EFileDialogType},
    data_source::{AssetFile, AssetFolder, DataSource, MeshItem, ModelViewData},
    editor_ui::EditorUI,
    model_loader::ModelLoader,
    project::Project,
    project_context::{EFolderUpdateType, ProjectContext},
    ui::{
        asset_view, content_browser, level_view,
        property_view::{self, ESelectedObject},
        texture_property_view, top_menu,
    },
};
use anyhow::{anyhow, Context};
use lazy_static::lazy_static;
use rs_core_minimal::path_ext::CanonicalizeSlashExt;
use rs_engine::{
    camera_input_event_handle::{CameraInputEventHandle, DefaultCameraInputEventHandle},
    content::{content_file_type::EContentFileType, texture::TextureFile},
    frame_sync::{EOptions, FrameSync},
    plugin::Plugin,
};
use rs_engine::{
    drawable::EDrawObjectType,
    file_type::EFileType,
    logger::{Logger, LoggerConfiguration},
    plugin_context::PluginContext,
    resource_manager::ResourceManager,
    static_virtual_texture_source::StaticVirtualTextureSource,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex},
};
use winit::{
    event::{ElementState, Event, MouseScrollDelta, WindowEvent},
    keyboard::KeyCode,
};

lazy_static! {
    static ref SUPPORT_ASSET_IMAGE_FILE_TYPES: HashSet<EFileType> = {
        let mut m = HashSet::new();
        m.insert(EFileType::Jpeg);
        m.insert(EFileType::Jpg);
        m.insert(EFileType::Png);
        m.insert(EFileType::Exr);
        m.insert(EFileType::Hdr);
        m
    };
    static ref SUPPORT_ASSET_MODEL_FILE_TYPES: HashSet<EFileType> = {
        let mut m = HashSet::new();
        m.insert(EFileType::Fbx);
        m.insert(EFileType::Glb);
        m.insert(EFileType::Blend);
        m.insert(EFileType::Dae);
        m
    };
    static ref SUPPORT_ASSET_FILE_TYPES: HashSet<EFileType> = {
        let mut m = HashSet::new();
        m.extend(SUPPORT_ASSET_IMAGE_FILE_TYPES.iter());
        m.extend(SUPPORT_ASSET_MODEL_FILE_TYPES.iter());
        m
    };
}

pub struct EditorContext {
    event_loop_proxy: winit::event_loop::EventLoopProxy<ECustomEventType>,
    engine: rs_engine::engine::Engine,
    egui_winit_state: Option<egui_winit::State>,
    data_source: DataSource,
    project_context: Option<ProjectContext>,
    draw_objects: HashMap<uuid::Uuid, EDrawObjectType>,
    virtual_key_code_states: HashMap<winit::keyboard::KeyCode, winit::event::ElementState>,
    editor_ui: EditorUI,
    plugin_context: Arc<Mutex<PluginContext>>,
    plugins: Vec<Box<dyn Plugin>>,
    frame_sync: FrameSync,
    model_loader: ModelLoader,
}

impl EditorContext {
    fn load_font() -> egui::FontDefinitions {
        let font_path = rs_core_minimal::file_manager::get_engine_resource(
            "Remote/Font/SourceHanSansHWSC/OTF/SimplifiedChineseHW/SourceHanSansHWSC-Regular.otf",
        );
        let font_data = match std::fs::read(font_path) {
            Ok(font_data) => font_data,
            Err(_) => {
                return egui::FontDefinitions::default();
            }
        };
        let mut font_definitions = egui::FontDefinitions::default().clone();
        font_definitions.families.clear();
        font_definitions.font_data.clear();
        font_definitions.font_data.insert(
            "SourceHanSansHWSC-Regular".to_owned(),
            egui::FontData::from_owned(font_data),
        );

        font_definitions.families.insert(
            egui::FontFamily::Monospace,
            vec!["SourceHanSansHWSC-Regular".to_owned()],
        );

        font_definitions.families.insert(
            egui::FontFamily::Proportional,
            vec!["SourceHanSansHWSC-Regular".to_owned()],
        );
        font_definitions
    }

    pub fn new(
        window: &winit::window::Window,
        event_loop_proxy: winit::event_loop::EventLoopProxy<ECustomEventType>,
    ) -> Self {
        rs_foundation::change_working_directory();
        let logger = Logger::new(LoggerConfiguration {
            is_write_to_file: true,
        });
        log::trace!(
            "Engine Root Dir: {:?}",
            rs_core_minimal::file_manager::get_engine_root_dir()
                .canonicalize_slash()
                .unwrap()
        );

        let window_size = window.inner_size();
        let scale_factor = 1.0f32;
        let window_width = window_size.width;
        let window_height = window_size.height;
        let egui_context = egui::Context::default();
        egui_context.set_fonts(Self::load_font());
        let style = egui::Style::default().clone();
        egui_context.set_style(style);
        let egui_winit_state = egui_winit::State::new(
            egui_context,
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
        );
        let artifact_reader = None;
        let mut engine = rs_engine::engine::Engine::new(
            window,
            window_width,
            window_height,
            scale_factor,
            logger,
            artifact_reader,
            ProjectContext::pre_process_shaders(),
        )
        .unwrap();
        Self::insert_cmds(&mut engine);

        let mut data_source = DataSource::new();
        data_source.console_cmds = Some(engine.get_console_cmds());
        let editor_ui = EditorUI::new(egui_winit_state.egui_ctx());

        let plugin_context = Arc::new(Mutex::new(PluginContext::new(
            egui_winit_state.egui_ctx().clone(),
        )));

        let frame_sync = FrameSync::new(EOptions::FPS(60.0));

        Self {
            event_loop_proxy,
            engine,
            egui_winit_state: Some(egui_winit_state),
            data_source,
            project_context: None,
            draw_objects: HashMap::new(),
            virtual_key_code_states: HashMap::new(),
            editor_ui,
            plugin_context,
            plugins: vec![],
            frame_sync,
            model_loader: ModelLoader::new(),
        }
    }

    fn insert_cmds(engine: &mut rs_engine::engine::Engine) {
        engine.insert_console_cmd(
            rs_engine::console_cmd::RS_TEST_KEY,
            rs_engine::console_cmd::ConsoleCmd {
                key: String::from(rs_engine::console_cmd::RS_TEST_KEY),
                value: rs_engine::console_cmd::EValue::I32(0),
            },
        );
    }

    pub fn handle_event(
        &mut self,
        window: &mut winit::window::Window,
        event: &Event<ECustomEventType>,
        event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<ECustomEventType>,
    ) {
        match event {
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    let input_mode = self.engine.get_input_mode();
                    DefaultCameraInputEventHandle::mouse_motion_handle(
                        &mut self.engine.get_camera_mut(),
                        *delta,
                        input_mode,
                        self.data_source.camera_motion_speed,
                    );
                }
                _ => {}
            },
            Event::UserEvent(event) => {
                self.process_custom_event(event, window);
            }
            Event::WindowEvent { event, .. } => {
                if let Some(egui_winit_state) = &mut self.egui_winit_state {
                    let _ = Some(egui_winit_state.on_window_event(window, event));
                }
                match event {
                    WindowEvent::CloseRequested => {
                        if let Some(egui_winit_state) = &mut self.egui_winit_state {
                            self.plugins.clear();
                            egui_winit_state.egui_ctx().memory_mut(|writer| {
                                writer.data.clear();
                            });
                            if let Some(ctx) = &mut self.project_context {
                                ctx.hot_reload.get_library_reload().lock().unwrap().clear();
                            }
                        }
                        event_loop_window_target.exit();
                    }
                    WindowEvent::Resized(size) => {
                        log::trace!("Window resized: {:?}", size);
                        self.engine
                            .get_camera_mut()
                            .set_window_size(size.width, size.height);
                        self.engine.resize(size.width, size.height);
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        MouseScrollDelta::LineDelta(_, up) => {
                            self.data_source.camera_movement_speed += up * 0.005;
                            self.data_source.camera_movement_speed =
                                self.data_source.camera_movement_speed.max(0.0);
                        }
                        MouseScrollDelta::PixelDelta(_) => todo!(),
                    },
                    WindowEvent::KeyboardInput {
                        device_id,
                        event,
                        is_synthetic,
                    } => {
                        self.process_keyboard_input(
                            window,
                            device_id,
                            event,
                            *is_synthetic,
                            event_loop_window_target,
                        );
                    }
                    WindowEvent::DroppedFile(file_path) => {
                        if let Some(project_context) = &self.project_context {
                            let target = project_context.get_asset_folder_path();
                            let result = std::fs::copy(
                                file_path,
                                target.join(file_path.file_name().unwrap()),
                            );
                            log::trace!("{:?}", result);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        self.engine.tick();
                        let is_minimized = window.is_minimized().unwrap_or(false);
                        let is_visible = window.is_visible().unwrap_or(true);
                        if !is_visible || is_minimized {
                            return;
                        }
                        if let Some(project_context) = &mut self.project_context {
                            if project_context.is_need_reload_plugin() {
                                self.try_load_plugin();
                            }
                        }
                        if let Some(project_context) = &mut self.project_context {
                            if let Some(folder_update_type) =
                                project_context.check_folder_notification()
                            {
                                match folder_update_type {
                                    EFolderUpdateType::Asset => {
                                        let asset_folder = Self::build_asset_folder(
                                            &project_context.get_asset_folder_path(),
                                        );
                                        log::trace!("Update asset folder. {:?}", asset_folder);
                                        self.data_source.asset_folder = Some(asset_folder.clone());
                                        self.data_source.current_asset_folder = Some(asset_folder);
                                    }
                                }
                            }
                        }

                        for (virtual_key_code, element_state) in &self.virtual_key_code_states {
                            let input_mode = self.engine.get_input_mode();
                            DefaultCameraInputEventHandle::keyboard_input_handle(
                                &mut self.engine.get_camera_mut(),
                                virtual_key_code,
                                element_state,
                                input_mode,
                                self.data_source.camera_movement_speed,
                            );
                        }

                        self.process_redraw_request(window);
                        let wait = self
                            .frame_sync
                            .tick()
                            .unwrap_or(std::time::Duration::from_secs_f32(1.0 / 60.0));
                        std::thread::sleep(wait);
                        window.request_redraw();

                        match self.engine.get_input_mode() {
                            rs_engine::input_mode::EInputMode::Game => {
                                window
                                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                                    .unwrap();
                                window.set_cursor_visible(false);
                            }
                            rs_engine::input_mode::EInputMode::UI => {
                                window
                                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                                    .unwrap();
                                window.set_cursor_visible(true);
                            }
                            rs_engine::input_mode::EInputMode::GameUI => todo!(),
                        }
                        self.data_source.input_mode = self.engine.get_input_mode();
                    }
                    WindowEvent::Destroyed => {}
                    _ => {}
                }
            }
            Event::NewEvents(_) => {}
            Event::LoopExiting => {}
            _ => {}
        }
    }

    fn try_load_plugin(&mut self) -> anyhow::Result<()> {
        if let Some(project_context) = self.project_context.as_mut() {
            project_context.reload()?;
            let lib = project_context.hot_reload.get_library_reload();
            let lib = lib.lock().unwrap();
            let func = lib.load_symbol::<rs_engine::plugin::signature::CreatePlugin>(
                rs_engine::plugin::symbol_name::CREATE_PLUGIN,
            )?;
            let plugin = func(Arc::clone(&self.plugin_context));
            self.plugins.push(plugin);
            log::trace!("Load plugin.");
        }
        Ok(())
    }

    fn build_asset_folder(path: &std::path::Path) -> AssetFolder {
        let mut folder = AssetFolder {
            name: path.file_stem().unwrap().to_str().unwrap().to_string(),
            path: path.to_path_buf(),
            files: vec![],
            folders: vec![],
        };

        for entry in walkdir::WalkDir::new(path).max_depth(1) {
            let Ok(entry) = entry else {
                continue;
            };
            if entry.path() == path {
                continue;
            }
            if entry.path().is_dir() {
                let path = entry.path();
                let sub_folder = Self::build_asset_folder(path);
                folder.folders.push(sub_folder);
            } else {
                let Some(extension) = entry.path().extension() else {
                    continue;
                };
                let extension = extension.to_ascii_lowercase();
                let extension = extension.to_str().unwrap();
                let Some(file_type) = EFileType::from_str(extension) else {
                    continue;
                };
                if !SUPPORT_ASSET_FILE_TYPES.contains(&file_type) {
                    continue;
                }
                let asset_file = AssetFile {
                    name: entry.file_name().to_str().unwrap().to_string(),
                    path: entry.path().to_path_buf(),
                };
                folder.files.push(asset_file);
            }
        }
        folder
    }

    fn process_keyboard_input(
        &mut self,
        window: &mut winit::window::Window,
        _: &winit::event::DeviceId,
        event: &winit::event::KeyEvent,
        _: bool,
        event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<ECustomEventType>,
    ) {
        let winit::keyboard::PhysicalKey::Code(virtual_keycode) = event.physical_key else {
            return;
        };

        self.virtual_key_code_states
            .insert(virtual_keycode, event.state);

        if Self::is_keys_pressed(&mut self.virtual_key_code_states, &[KeyCode::F1], true) {
            match self.engine.get_input_mode() {
                rs_engine::input_mode::EInputMode::Game => {
                    self.engine
                        .set_input_mode(rs_engine::input_mode::EInputMode::UI);
                }
                rs_engine::input_mode::EInputMode::UI => {
                    self.engine
                        .set_input_mode(rs_engine::input_mode::EInputMode::Game);
                }
                rs_engine::input_mode::EInputMode::GameUI => {
                    todo!()
                }
            }
        }

        if Self::is_keys_pressed(
            &mut self.virtual_key_code_states,
            &[KeyCode::Backquote],
            true,
        ) {
            self.data_source.is_console_cmds_view_open =
                !self.data_source.is_console_cmds_view_open;
        }

        if Self::is_keys_pressed(
            &mut self.virtual_key_code_states,
            &[KeyCode::AltLeft, KeyCode::F4],
            true,
        ) {
            event_loop_window_target.exit();
        }
    }

    fn is_keys_pressed(
        virtual_key_code_states: &mut HashMap<KeyCode, ElementState>,
        keys: &[KeyCode],
        is_consume: bool,
    ) -> bool {
        let mut states: HashMap<KeyCode, ElementState> = HashMap::new();
        for key in keys {
            if let Some(state) = virtual_key_code_states.get(key) {
                states.insert(*key, *state);
            }
        }
        if states.keys().len() == keys.len() {
            for state in states.values() {
                if *state == ElementState::Released {
                    return false;
                }
            }
            if is_consume {
                for key in states.keys() {
                    virtual_key_code_states.remove(key);
                }
            }
            true
        } else {
            false
        }
    }

    fn is_project_name_valid(name: &str) -> bool {
        if name.is_empty() || name.len() > 127 {
            return false;
        }
        let reg = regex::Regex::new("^[a-zA-Z]*$").unwrap();
        reg.is_match(name)
    }

    fn content_load_resources(
        engine: &mut rs_engine::engine::Engine,
        model_loader: &mut ModelLoader,
        project_context: &ProjectContext,
        files: Vec<EContentFileType>,
    ) {
        let asset_folder_path = project_context.get_asset_folder_path();
        for file in files {
            match file {
                EContentFileType::StaticMesh(static_mesh) => {
                    let file_path =
                        asset_folder_path.join(&static_mesh.borrow().asset_reference_relative_path);
                    model_loader.load(&file_path).unwrap();
                }
                EContentFileType::SkeletonMesh(skeleton_mesh) => {
                    let file_path =
                        asset_folder_path.join(&skeleton_mesh.borrow().get_relative_path());
                    model_loader.load(&file_path).unwrap();
                    model_loader.to_runtime_skin_mesh(
                        skeleton_mesh.clone(),
                        &asset_folder_path,
                        ResourceManager::default(),
                    );
                }
                EContentFileType::SkeletonAnimation(node_animation) => {
                    let file_path =
                        asset_folder_path.join(&node_animation.borrow().get_relative_path());
                    model_loader.load(&file_path).unwrap();
                    model_loader.to_runtime_skeleton_animation(
                        node_animation.clone(),
                        &asset_folder_path,
                        ResourceManager::default(),
                    );
                }
                EContentFileType::Skeleton(skeleton) => {
                    let file_path = asset_folder_path.join(&skeleton.borrow().get_relative_path());
                    model_loader.load(&file_path).unwrap();
                    let skeleton = model_loader.to_runtime_skeleton(
                        skeleton.clone(),
                        &asset_folder_path,
                        ResourceManager::default(),
                    );
                }
                EContentFileType::Texture(texture_file) => {
                    let texture_file = texture_file.borrow_mut();
                    let Some(image_reference) = &texture_file.image_reference else {
                        continue;
                    };
                    let abs_path = project_context
                        .get_asset_folder_path()
                        .join(image_reference);
                    let _ = engine.create_texture_from_path(&abs_path, texture_file.url.clone());

                    {
                        let url = texture_file.url.clone();
                        let Some(virtual_image_reference) = &texture_file.virtual_image_reference
                        else {
                            continue;
                        };
                        let path = project_context
                            .get_virtual_texture_cache_dir()
                            .join(virtual_image_reference);
                        let Ok(source) = StaticVirtualTextureSource::from_file(path, None) else {
                            continue;
                        };
                        engine.create_virtual_texture_source(url.clone(), Box::new(source));
                        log::trace!(
                            "Create virtual texture source, url: {}, {}",
                            url.as_str(),
                            virtual_image_reference
                        );
                    }
                }
                EContentFileType::Level(_) => {}
            }
        }
    }

    fn add_new_actors(
        engine: &mut rs_engine::engine::Engine,
        actors: Vec<Rc<RefCell<rs_engine::actor::Actor>>>,
        files: &[EContentFileType],
    ) {
        for actor in actors {
            let root_scene_node = &mut actor.borrow_mut().scene_node;
            match &mut root_scene_node.component {
                rs_engine::scene_node::EComponentType::SceneComponent(_) => todo!(),
                rs_engine::scene_node::EComponentType::StaticMeshComponent(_) => todo!(),
                rs_engine::scene_node::EComponentType::SkeletonMeshComponent(
                    skeleton_mesh_component,
                ) => {
                    skeleton_mesh_component.initialize(ResourceManager::default(), engine, files);
                }
            }
        }
    }

    fn open_project(
        &mut self,
        file_path: &Path,
        window: &mut winit::window::Window,
    ) -> anyhow::Result<()> {
        let project_context = ProjectContext::open(&file_path)?;
        window.set_title(&format!("Editor({})", project_context.project.project_name));
        let asset_folder_path = project_context.get_asset_folder_path();
        let asset_folder = Self::build_asset_folder(&asset_folder_path);
        self.editor_ui
            .set_asset_folder_path(Some(asset_folder_path.clone()));
        log::trace!("Update asset folder. {:?}", asset_folder);
        self.data_source.asset_folder = Some(asset_folder.clone());
        self.data_source.current_asset_folder = Some(asset_folder);

        let mut texture_files: Vec<Rc<RefCell<TextureFile>>> = vec![];
        self.data_source.content_data_source.current_folder =
            Some(project_context.project.content.clone());
        Self::content_load_resources(
            &mut self.engine,
            &mut self.model_loader,
            &project_context,
            project_context.project.content.borrow().files.clone(),
        );
        for file in &project_context.project.content.borrow().files.clone() {
            match file {
                EContentFileType::Texture(texture_file) => {
                    texture_files.push(texture_file.clone());
                }
                _ => {}
            }
        }

        self.engine
            .set_settings(project_context.project.settings.borrow().clone());
        self.data_source.project_settings = Some(project_context.project.settings.clone());

        {
            let binding = project_context.project.content.borrow();
            let find_level = binding
                .files
                .iter()
                .find(|x| match x {
                    EContentFileType::Level(_) => true,
                    _ => false,
                })
                .map(|x| match x {
                    EContentFileType::Level(level) => Some(level),
                    _ => None,
                })
                .flatten();

            self.data_source.level = find_level.cloned();

            if let Some(level) = find_level.cloned() {
                if let Some(folder) = &self.data_source.content_data_source.current_folder {
                    Self::add_new_actors(
                        &mut self.engine,
                        level.borrow().actors.clone(),
                        &folder.borrow().files,
                    );
                }
            }
        }

        self.project_context = Some(project_context);
        self.try_load_plugin();
        self.data_source
            .recent_projects
            .paths
            .insert(file_path.to_path_buf());
        self.data_source.recent_projects.save()?;
        Ok(())
    }

    fn process_custom_event(
        &mut self,
        event: &ECustomEventType,
        window: &mut winit::window::Window,
    ) {
        match event {
            ECustomEventType::OpenFileDialog(dialog_type) => match dialog_type {
                EFileDialogType::NewProject(name) => {
                    let result = (|| {
                        if !Self::is_project_name_valid(&name) {
                            return Err(anyhow!("Not a valid project name"));
                        }
                        let dialog = rfd::FileDialog::new();
                        let folder = dialog.pick_folder().ok_or(anyhow!("Fail to pick folder"))?;
                        log::trace!("Selected folder: {:?}", folder);
                        let project_file_path = Project::create_empty_project(&folder, name)?;
                        self.open_project(&project_file_path, window)?;
                        self.data_source.is_new_project_window_open = false;
                        Ok(())
                    })();
                    log::trace!("{:?}", result);
                }
                EFileDialogType::OpenProject => {
                    let dialog = rfd::FileDialog::new().add_filter("Project", &["rsproject"]);
                    let Some(file_path) = dialog.pick_file() else {
                        return;
                    };
                    log::trace!("Selected file: {:?}", file_path);
                    let result = self.open_project(&file_path, window);
                    log::trace!("{:?}", result);
                }

                EFileDialogType::IBL => {
                    let filter = [EFileType::Exr, EFileType::Hdr];
                    let dialog = rfd::FileDialog::new().add_filter("Image", &filter);
                    let Some(file_path) = dialog.pick_file() else {
                        return;
                    };
                    let url =
                        url::Url::parse(&format!("ibl://{}", uuid::Uuid::new_v4().to_string()))
                            .unwrap();
                    let save_dir = std::env::current_dir().unwrap().join("bake");
                    if !save_dir.exists() {
                        let _ = std::fs::create_dir(save_dir.clone());
                    }
                    self.engine.ibl_bake(
                        &file_path,
                        url,
                        self.data_source.ibl_bake_info.clone(),
                        Some(&save_dir),
                    );
                }
            },
        }
    }

    fn open_project_workspace(file_path: std::path::PathBuf) {
        std::thread::spawn(move || {
            let arg = file_path.to_str().unwrap();
            let _ = Command::new("Code")
                .arg(arg)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        });
    }

    fn open_model_file(&mut self, file_path: PathBuf) -> anyhow::Result<()> {
        let project_context = self
            .project_context
            .as_mut()
            .ok_or(anyhow!("Project context is null"))?;
        let active_level = self
            .data_source
            .level
            .clone()
            .ok_or(anyhow!("Active level is null"))?;
        let file_type = EFileType::from_path(&file_path)
            .context(format!("Invalid file type. {:?}", file_path))?;
        if !SUPPORT_ASSET_MODEL_FILE_TYPES.contains(&file_type) {
            return Err(anyhow!("Not support file type. {:?}", file_type));
        }
        let asset_reference = file_path
            .strip_prefix(project_context.get_asset_folder_path())?
            .to_str()
            .unwrap();

        let load_result = self
            .model_loader
            .load_from_file_as_actor(&file_path, asset_reference.to_string())?;

        let content = project_context.project.content.clone();
        let mut content = content.borrow_mut();
        let mut add_files: Vec<EContentFileType> = vec![];
        for static_mesh in &load_result.static_meshes {
            add_files.push(EContentFileType::StaticMesh(static_mesh.clone()));
        }
        for skeleton_meshe in &load_result.skeleton_meshes {
            add_files.push(EContentFileType::SkeletonMesh(skeleton_meshe.clone()));
        }
        for node_animation in &load_result.node_animations {
            add_files.push(EContentFileType::SkeletonAnimation(node_animation.clone()));
        }
        if let Some(skeleton) = &load_result.skeleton {
            add_files.push(EContentFileType::Skeleton(skeleton.clone()));
        }
        Self::content_load_resources(
            &mut self.engine,
            &mut self.model_loader,
            project_context,
            add_files.clone(),
        );
        content.files.append(&mut add_files);

        Self::add_new_actors(
            &mut self.engine,
            vec![load_result.actor.clone()],
            &content.files,
        );

        active_level.borrow_mut().actors.push(load_result.actor);

        let mesh_clusters = ModelLoader::load_from_file(&file_path, &[])?;
        self.data_source.is_model_hierarchy_open = true;
        let mut items: Vec<Rc<MeshItem>> = vec![];
        for mesh_cluster in mesh_clusters {
            let item = MeshItem {
                name: mesh_cluster.name,
                childs: vec![],
            };
            items.push(Rc::new(item));
        }
        let model_view_data = ModelViewData {
            mesh_items: items,
            file_path,
        };
        self.data_source.model_view_data = Some(model_view_data);
        Ok(())
    }

    fn process_redraw_request(&mut self, window: &mut winit::window::Window) {
        if let Some(project_context) = &mut self.project_context {
            if let Some(active_level) = self.data_source.level.clone() {
                for actor in active_level.borrow().actors.clone() {
                    let root_scene_node = &mut actor.borrow_mut().scene_node;
                    match &mut root_scene_node.component {
                        rs_engine::scene_node::EComponentType::SceneComponent(_) => todo!(),
                        rs_engine::scene_node::EComponentType::StaticMeshComponent(_) => todo!(),
                        rs_engine::scene_node::EComponentType::SkeletonMeshComponent(
                            skeleton_mesh_component,
                        ) => {
                            skeleton_mesh_component
                                .update(self.engine.get_game_time(), &mut self.engine);

                            for draw_object in skeleton_mesh_component.get_draw_objects() {
                                self.engine.draw2(draw_object);
                            }
                        }
                    }
                }
            }
        }

        for (_, draw_object) in self.draw_objects.iter_mut() {
            self.engine.update_draw_object(draw_object);
            self.engine.draw2(draw_object);
        }
        self.process_ui(window);

        if let Some(plugin) = self.plugins.last_mut() {
            plugin.tick();
        }

        let gui_render_output = (|| {
            let Some(egui_winit_state) = &mut self.egui_winit_state else {
                return None;
            };
            let full_output = egui_winit_state.egui_ctx().end_frame();

            egui_winit_state.handle_platform_output(window, full_output.platform_output.clone());

            let gui_render_output = rs_render::egui_render::EGUIRenderOutput {
                textures_delta: full_output.textures_delta,
                clipped_primitives: egui_winit_state
                    .egui_ctx()
                    .tessellate(full_output.shapes, full_output.pixels_per_point),
            };
            Some(gui_render_output)
        })();

        if let Some(gui_render_output) = gui_render_output {
            self.engine.redraw(gui_render_output);
        }

        self.engine.present();
    }

    pub fn prepreocess_shader() -> anyhow::Result<()> {
        let buildin_shaders = rs_render::global_shaders::get_buildin_shaders();
        let output_path =
            rs_core_minimal::file_manager::get_engine_root_dir().join("rs_editor/target/shaders");
        if !output_path.exists() {
            std::fs::create_dir(output_path.clone())
                .context(anyhow!("Can not create dir {:?}", output_path))?;
        }

        let mut compile_commands = vec![];
        for buildin_shader in buildin_shaders {
            let description = buildin_shader.get_shader_description();
            let name = buildin_shader.get_name();
            let processed_code = rs_shader_compiler::pre_process::pre_process(
                &description.shader_path,
                description.include_dirs.iter(),
                description.definitions.iter(),
            )?;
            let filepath = output_path.join(name);
            std::fs::write(filepath.clone(), processed_code)
                .context(anyhow!("Can not write to file {:?}", filepath))?;

            let compile_command = buildin_shader.as_ref().to_compile_command();
            compile_commands.push(compile_command);
        }
        let output_path = rs_core_minimal::file_manager::get_engine_root_dir().join(".vscode");
        if !output_path.exists() {
            std::fs::create_dir(output_path.clone())
                .context(anyhow!("Can not create dir {:?}", output_path))?;
        }
        let target_path = output_path.join("shader_compile_commands.json");
        let _ = std::fs::write(
            target_path.clone(),
            serde_json::to_string(&compile_commands)?,
        )
        .context(anyhow!("Can not write to file {:?}", target_path))?;
        Ok(())
    }

    fn process_ui(&mut self, window: &mut winit::window::Window) {
        let Some(egui_winit_state) = &mut self.egui_winit_state else {
            return;
        };
        let new_input = egui_winit_state.take_egui_input(window);
        egui_winit_state.egui_ctx().begin_frame(new_input);
        egui_winit_state.egui_ctx().clear_animations();

        let click_event = self
            .editor_ui
            .build(egui_winit_state.egui_ctx(), &mut self.data_source);

        if let Some(menu_event) = click_event.menu_event {
            match menu_event {
                top_menu::EClickEventType::NewProject(projevt_name) => {
                    let _ = self
                        .event_loop_proxy
                        .send_event(ECustomEventType::OpenFileDialog(
                            EFileDialogType::NewProject(projevt_name.clone()),
                        ));
                }
                top_menu::EClickEventType::OpenProject => {
                    let _ = self
                        .event_loop_proxy
                        .send_event(ECustomEventType::OpenFileDialog(
                            EFileDialogType::OpenProject,
                        ));
                }
                top_menu::EClickEventType::OpenRecentProject(file_path) => {
                    let result = self.open_project(&file_path, window);
                    log::trace!("Open project {result:?}");
                }
                top_menu::EClickEventType::SaveProject => {
                    if let Some(project_context) = self.project_context.as_ref() {
                        let save_status = project_context.save();
                        log::trace!("Save project: {:?}", save_status);
                    }
                }
                top_menu::EClickEventType::Export => {
                    if let Some(project_context) = self.project_context.as_mut() {
                        let result = project_context.export(&mut self.model_loader);
                        log::trace!("{:?}", result);
                    }
                }
                top_menu::EClickEventType::OpenVisualStudioCode => {
                    if let Some(project_context) = &self.project_context {
                        let path = project_context.get_project_folder_path();
                        Self::open_project_workspace(path);
                    }
                }
                top_menu::EClickEventType::Build(build_config) => {
                    let result = (|project_context: Option<&mut ProjectContext>| {
                        let project_context =
                            project_context.ok_or(anyhow!("project_context is null"))?;
                        let artifact_file_path = project_context.export(&mut self.model_loader)?;
                        let folder_path =
                            project_context.create_build_folder_if_not_exist(&build_config)?;
                        let target = rs_core_minimal::file_manager::get_engine_root_dir()
                            .join("rs_desktop_standalone/target");
                        let exe: PathBuf;
                        match build_config.build_type {
                            EBuildType::Debug => {
                                exe = target.join("debug/rs_desktop_standalone.exe");
                            }
                            EBuildType::Release => {
                                exe = target.join("release/rs_desktop_standalone.exe");
                            }
                        }
                        let to = folder_path.join("rs_desktop_standalone.exe");
                        Self::copy_file_and_log(exe, to)?;
                        let file_name = artifact_file_path
                            .file_name()
                            .ok_or(anyhow!("No file name"))?;
                        let to = folder_path.join(file_name);
                        Self::copy_file_and_log(artifact_file_path, to)?;
                        Ok::<(), anyhow::Error>(())
                    })(self.project_context.as_mut());
                    log::trace!("{:?}", result);
                }
                top_menu::EClickEventType::OpenWindow(window_type) => match window_type {
                    top_menu::EWindowType::Asset => {
                        self.data_source.is_asset_folder_open = true;
                    }
                    top_menu::EWindowType::Content => {
                        self.data_source.content_data_source.is_open = true;
                    }
                    top_menu::EWindowType::Property => {
                        self.data_source.property_view_data_source.is_open = true;
                    }
                    top_menu::EWindowType::Level => {
                        self.data_source.is_level_view_open = true;
                    }
                    top_menu::EWindowType::ComsoleCmds => {
                        self.data_source.is_console_cmds_view_open = true;
                    }
                },
                top_menu::EClickEventType::Tool(tool_type) => match tool_type {
                    top_menu::EToolType::IBL => {
                        let _ = self
                            .event_loop_proxy
                            .send_event(ECustomEventType::OpenFileDialog(EFileDialogType::IBL));
                    }
                    top_menu::EToolType::DebugShader => {
                        Self::prepreocess_shader();
                    }
                },
                top_menu::EClickEventType::OpenProjectSettings => {
                    if self.project_context.is_some() {
                        self.data_source.project_settings_open = true;
                    }
                }
                top_menu::EClickEventType::ViewMode(mode) => {
                    self.engine.set_view_mode(mode);
                }
            }
        }
        if let Some(click_aseet) = click_event.click_aseet {
            match click_aseet {
                asset_view::EClickItemType::Folder(folder) => {
                    self.data_source.current_asset_folder = Some(folder);
                }
                asset_view::EClickItemType::File(asset_file) => {
                    self.data_source.highlight_asset_file = Some(asset_file.clone());
                    let result = self.open_model_file(asset_file.path.clone());
                    log::trace!("{:?}", result);
                }
                asset_view::EClickItemType::Back => todo!(),
                asset_view::EClickItemType::SingleClickFile(asset_file) => {
                    self.data_source.highlight_asset_file = Some(asset_file)
                }
                asset_view::EClickItemType::CreateTexture(asset_file) => {
                    if let Some(project_context) = self.project_context.as_mut() {
                        let asset_folder_path = project_context.get_asset_folder_path();
                        let image_reference: PathBuf = {
                            if asset_file.path.starts_with(asset_folder_path.clone()) {
                                asset_file
                                    .path
                                    .strip_prefix(asset_folder_path)
                                    .unwrap()
                                    .to_path_buf()
                            } else {
                                asset_file.path
                            }
                        };
                        if let Some(current_folder) =
                            &self.data_source.content_data_source.current_folder
                        {
                            let mut current_folder = current_folder.borrow_mut();
                            let folder_url = current_folder.get_url();
                            let url = folder_url.join(&asset_file.name).unwrap();
                            let mut texture_file = TextureFile::new(asset_file.name, url);
                            texture_file.image_reference = Some(image_reference);
                            log::trace!("Create texture: {:?}", &texture_file.url.as_str());
                            current_folder.files.push(EContentFileType::Texture(Rc::new(
                                RefCell::new(texture_file),
                            )));
                        }
                    }
                }
            }
        }

        if let Some(click_actor) = click_event.click_actor {
            match click_actor {
                level_view::EClickEventType::Actor(actor) => {
                    self.data_source.property_view_data_source.is_open = true;
                    self.data_source.property_view_data_source.selected_actor = Some(actor.clone());
                    self.data_source.property_view_data_source.selected_object =
                        Some(ESelectedObject::Actor(actor.clone()));
                }
            }
        }
        if let Some(property_event) = click_event.property_event {
            match property_event {
                property_view::EClickEventType::Node(property_event) => {}
                property_view::EClickEventType::TextureFile(event) => match event {
                    texture_property_view::EClickEventType::IsVirtualTexture(
                        is_virtual_texture,
                    ) => {
                        (|| {
                            if !is_virtual_texture {
                                return;
                            }
                            let Some(selected_object) =
                                &self.data_source.property_view_data_source.selected_object
                            else {
                                return;
                            };
                            let ESelectedObject::TextureFile(texture_file) = selected_object else {
                                return;
                            };
                            let Some(project_context) = &self.project_context else {
                                return;
                            };
                            let Ok(virtual_texture_cache_dir) =
                                project_context.try_create_virtual_texture_cache_dir()
                            else {
                                return;
                            };
                            let asset_folder = &project_context.get_asset_folder_path();
                            let Ok(virtual_cache_name) = texture_file
                                .borrow()
                                .get_pref_virtual_cache_name(asset_folder)
                            else {
                                return;
                            };
                            let result = texture_file.borrow_mut().create_virtual_texture_cache(
                                asset_folder,
                                &virtual_texture_cache_dir.join(virtual_cache_name.clone()),
                                Some(rs_artifact::EEndianType::Little),
                                256,
                            );
                            if result.is_ok() {
                                log::trace!("virtual_cache_name: {}", virtual_cache_name);
                                texture_file.borrow_mut().virtual_image_reference =
                                    Some(virtual_cache_name);
                            }
                        })();
                    }
                },
            }
        }

        if let Some(gizmo_result) = &click_event.gizmo_result {}
        if let Some(event) = click_event.content_browser_event {
            if let Some(current_folder) = &self.data_source.content_data_source.current_folder {
                match event {
                    content_browser::EClickEventType::CreateFolder => {
                        let new_folder_name = &self.data_source.content_data_source.new_folder_name;
                        let names: Vec<String> = current_folder
                            .borrow()
                            .folders
                            .iter()
                            .map(|x| x.borrow().name.clone())
                            .collect();
                        if !names.contains(new_folder_name) {
                            let new_folder =
                                ContentFolder::new(new_folder_name, Some(current_folder.clone()));
                            current_folder
                                .borrow_mut()
                                .folders
                                .push(Rc::new(RefCell::new(new_folder)));
                        }
                    }
                    content_browser::EClickEventType::Back => {
                        let parent_folder = current_folder.borrow().parent_folder.clone();
                        if let Some(parent_folder) = parent_folder {
                            self.data_source.content_data_source.current_folder =
                                Some(parent_folder.clone());
                        }
                    }
                    content_browser::EClickEventType::OpenFolder(folder) => {
                        self.data_source.content_data_source.current_folder = Some(folder);
                    }
                    content_browser::EClickEventType::OpenFile(file) => match file {
                        EContentFileType::StaticMesh(_) => todo!(),
                        EContentFileType::SkeletonMesh(_) => todo!(),
                        EContentFileType::SkeletonAnimation(_) => todo!(),
                        EContentFileType::Skeleton(_) => todo!(),
                        EContentFileType::Texture(_) => todo!(),
                        EContentFileType::Level(_) => todo!(),
                    },
                    content_browser::EClickEventType::SingleClickFile(file) => {
                        self.data_source.content_data_source.highlight_file = Some(file.clone());
                        match file {
                            EContentFileType::StaticMesh(_) => todo!(),
                            EContentFileType::SkeletonMesh(_) => todo!(),
                            EContentFileType::SkeletonAnimation(_) => todo!(),
                            EContentFileType::Skeleton(_) => todo!(),
                            EContentFileType::Texture(texture_file) => {
                                self.data_source.property_view_data_source.selected_object =
                                    Some(ESelectedObject::TextureFile(texture_file));
                            }
                            EContentFileType::Level(_) => todo!(),
                        }
                    }
                }
            }
        }
    }

    pub fn copy_file_and_log<P: AsRef<Path> + Clone + Debug>(
        from: P,
        to: P,
    ) -> std::io::Result<u64> {
        let result = std::fs::copy(from.clone(), to.clone());
        match &result {
            Ok(_) => {
                log::trace!("Copy {:?} to {:?}", from, to);
            }
            Err(err) => {
                log::warn!("{}, can not copy {:?} to {:?}", err, from, to);
            }
        }
        result
    }
}

#[cfg(test)]
mod test {
    use super::EditorContext;

    #[test]
    fn test_case() {
        assert_eq!(EditorContext::is_project_name_valid("name1"), false);
        assert_eq!(EditorContext::is_project_name_valid("1name"), false);
        assert_eq!(EditorContext::is_project_name_valid("*name"), false);
        assert_eq!(EditorContext::is_project_name_valid("name*"), false);
        assert_eq!(EditorContext::is_project_name_valid("*****"), false);
        assert_eq!(EditorContext::is_project_name_valid("11111"), false);
        assert_eq!(EditorContext::is_project_name_valid("na me"), false);

        assert_eq!(EditorContext::is_project_name_valid(""), false);
        assert_eq!(
            EditorContext::is_project_name_valid(&vec!['a'; 999].iter().collect::<String>()),
            false
        );

        assert_eq!(EditorContext::is_project_name_valid("name"), true);
    }
}
