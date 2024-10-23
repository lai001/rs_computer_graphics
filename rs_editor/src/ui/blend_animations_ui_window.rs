use super::{
    misc::{self, render_combo_box_not_null, update_window_with_input_mode},
    ui_window::UIWindow,
};
use crate::{editor_context::EWindowType, windows_manager::WindowsManager};
use anyhow::anyhow;
use egui_winit::State;
use rs_engine::{
    content::{blend_animations::BlendAnimations, skeleton_mesh::SkeletonMesh},
    engine::Engine,
    frame_sync::{EOptions, FrameSync},
    input_mode::EInputMode,
    player_viewport::PlayerViewport,
    resource_manager::ResourceManager,
    skeleton_animation_provider::SkeletonAnimationBlendType,
    skeleton_mesh_component::SkeletonMeshComponent,
};
use rs_foundation::new::SingleThreadMutType;
use std::collections::HashMap;

pub struct BlendAnimationUIWindow {
    pub egui_winit_state: State,
    frame_sync: FrameSync,
    virtual_key_code_states: HashMap<winit::keyboard::KeyCode, winit::event::ElementState>,
    input_mode: EInputMode,
    player_view_port: PlayerViewport,
    blend_animation: SingleThreadMutType<BlendAnimations>,
    preview_skeleton_mesh_component: Option<SkeletonMeshComponent>,
    preview_skeleton_url: Option<url::Url>,
    skeleton_meshes: Vec<SingleThreadMutType<SkeletonMesh>>,
    pub content: SingleThreadMutType<crate::content_folder::ContentFolder>,
    start: std::time::Instant,
}

impl BlendAnimationUIWindow {
    pub fn new(
        context: egui::Context,
        window_manager: &mut WindowsManager,
        event_loop_window_target: &winit::event_loop::ActiveEventLoop,
        engine: &mut Engine,
        content: SingleThreadMutType<crate::content_folder::ContentFolder>,
        blend_animation: SingleThreadMutType<BlendAnimations>,
    ) -> anyhow::Result<BlendAnimationUIWindow> {
        let window_context = window_manager
            .spwan_new_window(EWindowType::BlendAnimation, event_loop_window_target)?;
        let window_id = window_context.get_id();
        let window = &*window_context.window.borrow();
        let width = window.inner_size().width;
        let height = window.inner_size().height;

        engine
            .set_new_window(
                window_context.get_id(),
                window,
                window_context.get_width(),
                window_context.get_height(),
                window.scale_factor() as f32,
            )
            .map_err(|err| anyhow!("{err}"))?;
        let viewport_id = egui::ViewportId::from_hash_of(window_context.get_id());

        let mut egui_winit_state = egui_winit::State::new(
            context,
            viewport_id,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        egui_winit_state.egui_input_mut().viewport_id = viewport_id;
        egui_winit_state.egui_input_mut().viewports =
            std::iter::once((viewport_id, Default::default())).collect();

        let frame_sync = FrameSync::new(EOptions::FPS(60.0));

        let input_mode = EInputMode::UI;
        update_window_with_input_mode(window, input_mode);

        let player_view_port =
            PlayerViewport::from_window_surface(window_id, width, height, engine, input_mode, true);

        Ok(BlendAnimationUIWindow {
            egui_winit_state,
            frame_sync,
            virtual_key_code_states: HashMap::new(),
            input_mode,
            player_view_port,
            blend_animation,
            preview_skeleton_mesh_component: None,
            preview_skeleton_url: None,
            content,
            skeleton_meshes: vec![],
            start: std::time::Instant::now(),
        })
    }

    fn collect_skeleton_urls(&self) -> Vec<url::Url> {
        let files = self.content.borrow().files.clone();
        let mut urls = vec![];
        for file in files {
            if let rs_engine::content::content_file_type::EContentFileType::Skeleton(_) = file {
                let url = file.get_url();
                urls.push(url);
            }
        }
        urls
    }

    fn collect_animation_urls(&self) -> Vec<url::Url> {
        let files = self.content.borrow().files.clone();
        let mut urls = vec![];
        for file in files {
            if let rs_engine::content::content_file_type::EContentFileType::SkeletonAnimation(_) =
                file
            {
                let url = file.get_url();
                urls.push(url);
            }
        }
        urls
    }

    fn collect_skeleton_meshes_with_skeleton_url(
        &self,
        skeleton_url: &url::Url,
    ) -> Vec<SingleThreadMutType<SkeletonMesh>> {
        let files = self.content.borrow().files.clone();
        let skeleton_meshes: Vec<SingleThreadMutType<SkeletonMesh>> = files
            .iter()
            .filter_map(|x| {
                if let rs_engine::content::content_file_type::EContentFileType::SkeletonMesh(
                    skeleton_mesh,
                ) = x
                {
                    if &skeleton_mesh.borrow().skeleton_url == skeleton_url {
                        Some(skeleton_mesh.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        skeleton_meshes
    }
}

impl UIWindow for BlendAnimationUIWindow {
    fn on_device_event(&mut self, device_event: &winit::event::DeviceEvent) {
        let _ = device_event;
        self.player_view_port
            .on_input(rs_engine::input_type::EInputType::Device(device_event));
    }

    fn on_window_event(
        &mut self,
        window_id: isize,
        window: &mut winit::window::Window,
        event: &winit::event::WindowEvent,
        event_loop_window_target: &winit::event_loop::ActiveEventLoop,
        engine: &mut rs_engine::engine::Engine,
        window_manager: &mut crate::windows_manager::WindowsManager,
        is_request_close: &mut bool,
    ) {
        let _ = event_loop_window_target;
        let _ = window_manager;
        let _ = is_request_close;
        let _ = self.egui_winit_state.on_window_event(window, event);
        match event {
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                let winit::keyboard::PhysicalKey::Code(key_code) = event.physical_key else {
                    return;
                };
                self.virtual_key_code_states.insert(key_code, event.state);
                self.player_view_port
                    .on_input(rs_engine::input_type::EInputType::KeyboardInput(
                        &self.virtual_key_code_states,
                    ));
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                self.player_view_port
                    .on_input(rs_engine::input_type::EInputType::MouseWheel(delta));
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Right {
                    match state {
                        winit::event::ElementState::Pressed => {
                            self.input_mode = EInputMode::Game;
                        }
                        winit::event::ElementState::Released => {
                            self.input_mode = EInputMode::UI;
                        }
                    }
                    update_window_with_input_mode(window, self.input_mode);
                    self.player_view_port.set_input_mode(self.input_mode);
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                let time = std::time::Instant::now() - self.start;
                self.frame_sync.sync(60.0);
                engine.window_redraw_requested_begin(window_id);
                self.player_view_port
                    .on_input(rs_engine::input_type::EInputType::KeyboardInput(
                        &self.virtual_key_code_states,
                    ));
                self.player_view_port.update_global_constants(engine);

                engine.present_player_viewport(&mut self.player_view_port);
                let candidate_items = self.collect_skeleton_urls();
                let ctx = self.egui_winit_state.egui_ctx().clone();
                misc::ui_begin(&mut self.egui_winit_state, window);
                egui::Window::new("")
                    .default_open(true)
                    .open(&mut true)
                    .show(&ctx, |ui| {
                        let _ = ui;
                        let mut current_value = self.preview_skeleton_url.as_ref();
                        let is_changed = misc::render_combo_box(
                            ui,
                            "Preview skeleton",
                            &mut current_value,
                            &candidate_items,
                        );
                        if is_changed {
                            let new = current_value.cloned();
                            self.preview_skeleton_url = new;
                            if let Some(skeleton_url) = &self.preview_skeleton_url {
                                self.skeleton_meshes =
                                    self.collect_skeleton_meshes_with_skeleton_url(skeleton_url);

                                let skeleton_mesh_urls = self
                                    .skeleton_meshes
                                    .iter()
                                    .map(|x| x.borrow().url.clone())
                                    .collect();
                                let mut skeleton_mesh_component = SkeletonMeshComponent::new(
                                    format!("PreviewSkeletonMesh"),
                                    Some(skeleton_url.clone()),
                                    skeleton_mesh_urls,
                                    Some(self.blend_animation.borrow().url.clone()),
                                    None,
                                    glam::Mat4::IDENTITY,
                                );
                                let files = &self.content.borrow().files;
                                let resource_manager = ResourceManager::default();
                                skeleton_mesh_component.initialize(
                                    resource_manager,
                                    engine,
                                    files,
                                    &mut self.player_view_port,
                                );
                                self.preview_skeleton_mesh_component =
                                    Some(skeleton_mesh_component);
                            }
                        }

                        let is_add = ui.button("+").clicked();
                        if is_add {
                            if let Some(animation_url) =
                                self.collect_animation_urls().first().cloned()
                            {
                                let added_channel = rs_engine::content::blend_animations::Channel {
                                    animation_url,
                                    blend_type: SkeletonAnimationBlendType::Combine(1.0),
                                    time_range: 0.0..=3.0,
                                };
                                let mut blend_animation = self.blend_animation.borrow_mut();
                                blend_animation.channels.push(added_channel);
                            }
                        }

                        let mut blend_animation = self.blend_animation.borrow_mut();
                        let channels = &mut blend_animation.channels;
                        for (index, channel) in channels.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                let current_value = &mut channel.animation_url;
                                render_combo_box_not_null(
                                    ui,
                                    &format!("Animation {}", index),
                                    current_value,
                                    self.collect_animation_urls(),
                                );
                                match &mut channel.blend_type {
                                    SkeletonAnimationBlendType::Combine(factor) => {
                                        ui.add(
                                            egui::DragValue::new(factor)
                                                .speed(0.01)
                                                .prefix("factor: "),
                                        );
                                    }
                                }
                            });
                        }
                    });
                let gui_render_output = misc::ui_end(&mut self.egui_winit_state, window, window_id);

                if let Some(preview_skeleton_mesh_component) =
                    self.preview_skeleton_mesh_component.as_mut()
                {
                    preview_skeleton_mesh_component.update(time.as_secs_f32(), engine);
                    let mut draw_objects: Vec<_> = preview_skeleton_mesh_component
                        .get_draw_objects()
                        .iter()
                        .map(|x| (*x).clone())
                        .collect();
                    for draw_object in draw_objects.iter_mut() {
                        self.player_view_port
                            .update_draw_object(engine, draw_object);
                        draw_object.switch_player_viewport(&self.player_view_port);
                    }
                    self.player_view_port.append_to_draw_list(&draw_objects);
                }
                engine.present_player_viewport(&mut self.player_view_port);

                engine.send_render_command(rs_render::command::RenderCommand::UiOutput(
                    gui_render_output,
                ));
                engine.window_redraw_requested_end(window_id);
                window.request_redraw();
            }
            _ => {}
        }
    }
}
