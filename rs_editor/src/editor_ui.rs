use crate::data_source::{DataSource, MeshItem};
use crate::editor_ui::load::ImageLoader;
use crate::thumbnail_cache::ThumbnailCache;
use crate::ui::content_item_property_view::ContentItemPropertyView;
use crate::ui::debug_textures_view::{self, DebugTexturesView};
use crate::ui::gizmo_view::GizmoView;
use crate::ui::object_property_view::{self, ESelectedObjectType, ObjectPropertyView};
use crate::ui::top_menu::TopMenu;
use crate::ui::{
    asset_view, console_cmds_view, content_browser, gizmo_settings, level_view, project_settings,
    top_menu,
};
use egui::*;
use rs_engine::input_mode::EInputMode;
use std::sync::Arc;
use std::{path::PathBuf, rc::Rc};
use transform_gizmo_egui::math::Transform;
use transform_gizmo_egui::GizmoResult;

#[derive(Debug)]
pub struct ClickMeshItem {
    pub file_path: PathBuf,
    pub item: Rc<MeshItem>,
}

pub struct GizmoEvent {
    pub selected_object: ESelectedObjectType,
    pub gizmo_result: Option<(GizmoResult, Vec<Transform>)>,
}

#[derive(Default)]
pub struct ClickEvent {
    pub click_actor: Option<level_view::EClickEventType>,
    pub mesh_item: Option<ClickMeshItem>,
    pub click_aseet: Option<asset_view::EClickItemType>,
    pub menu_event: Option<top_menu::EClickEventType>,
    pub content_browser_event: Option<content_browser::EClickEventType>,
    pub debug_textures_view_event: Option<debug_textures_view::EClickEventType>,
    pub project_settings_event: Option<project_settings::EEventType>,
    pub object_property_view_event: Option<object_property_view::EEventType>,
    pub gizmo_event: Option<GizmoEvent>,
}

pub struct EditorUI {
    _image_loader: Option<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    _svg_loader: Option<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    project_folder_path: Option<PathBuf>,
    top_menu: TopMenu,
    pub gizmo_view: GizmoView,
    pub egui_context: Context,
    pub content_item_property_view: ContentItemPropertyView,
    pub object_property_view: ObjectPropertyView,
    pub debug_textures_view: DebugTexturesView,
    thumbnail_cache: ThumbnailCache,
}

impl EditorUI {
    pub fn new(context: &Context) -> Self {
        let image_loader_id = "egui_extras::loaders::image_loader::ImageCrateLoader";
        let svg_loader_id = "egui_extras::loaders::svg_loader::SvgLoader";
        let mut image_loader = None;
        let mut svg_loader = None;
        egui_extras::install_image_loaders(context);
        for item in context.loaders().image.lock().iter() {
            if item.id() == image_loader_id {
                image_loader = Some(item.clone());
            }
            if item.id() == svg_loader_id {
                svg_loader = Some(item.clone());
            }
        }
        EditorUI {
            _image_loader: image_loader,
            _svg_loader: svg_loader,
            project_folder_path: None,
            top_menu: TopMenu {
                new_project_name: String::new(),
            },
            gizmo_view: GizmoView::default(),
            egui_context: context.clone(),
            content_item_property_view: ContentItemPropertyView::new(),
            object_property_view: ObjectPropertyView::new(),
            debug_textures_view: DebugTexturesView::new(),
            thumbnail_cache: ThumbnailCache::new(),
        }
    }

    pub fn set_project_folder_path(&mut self, project_folder_path: Option<PathBuf>) {
        self.project_folder_path = project_folder_path;
    }

    pub fn build(&mut self, context: &Context, data_source: &mut DataSource) -> ClickEvent {
        let mut click = ClickEvent::default();
        click.menu_event = self.top_menu.draw(context, data_source);

        Self::model_hierarchy_window(context, data_source, &mut click);
        if let Some(level) = &data_source.level {
            let window = Self::new_window("Level", data_source.input_mode);
            click.click_actor = crate::ui::level_view::draw(
                window,
                context,
                &mut data_source.is_level_view_open,
                &level.as_ref().borrow(),
            );
        }
        let window = Self::new_window("Asset", data_source.input_mode);
        click.click_aseet = asset_view::draw(
            window,
            context,
            &mut data_source.is_asset_folder_open,
            data_source.current_asset_folder.as_ref(),
            data_source.highlight_asset_file.as_ref(),
            &mut self.thumbnail_cache,
        );

        if let Some(selected_object) = self.object_property_view.selected_object.as_ref() {
            macro_rules! gizmo {
                ($component:ident) => {
                    let mut component = $component.borrow_mut();
                    let model_matrix = component.get_interactive_transformation();
                    let gizmo_result = self.gizmo_view.draw(
                        context,
                        data_source.camera_view_matrix,
                        data_source.camera_projection_matrix,
                        *model_matrix,
                    );
                    if let Some((_, transforms)) = gizmo_result {
                        let transform = transforms[0];
                        *model_matrix = glam::DMat4::from_scale_rotation_translation(
                            transform.scale.into(),
                            transform.rotation.into(),
                            transform.translation.into(),
                        )
                        .as_mat4();
                    }
                };
            }

            match selected_object {
                ESelectedObjectType::Actor(_) => {}
                ESelectedObjectType::SceneComponent(component) => {
                    gizmo!(component);
                }
                ESelectedObjectType::StaticMeshComponent(component) => {
                    let mut component = component.borrow_mut();
                    let model_matrix = component.get_interactive_transformation();
                    let gizmo_result = self.gizmo_view.draw(
                        context,
                        data_source.camera_view_matrix,
                        data_source.camera_projection_matrix,
                        *model_matrix,
                    );
                    click.gizmo_event = Some(GizmoEvent {
                        selected_object: selected_object.clone(),
                        gizmo_result,
                    });
                }
                ESelectedObjectType::SkeletonMeshComponent(component) => {
                    gizmo!(component);
                }
                ESelectedObjectType::DirectionalLight(light) => {
                    gizmo!(light);
                }
            }
        }
        let window = Self::new_window("Gizmo Settings", data_source.input_mode);
        gizmo_settings::draw(
            window,
            context,
            &mut self.gizmo_view.visuals,
            &mut self.gizmo_view.gizmo_mode,
            &mut self.gizmo_view.gizmo_orientation,
            &mut self.gizmo_view.custom_highlight_color,
        );
        if let Some(project_settings) = data_source.project_settings.clone() {
            let window = Self::new_window("Project Settings", data_source.input_mode);
            click.project_settings_event = crate::ui::project_settings::draw(
                window,
                context,
                &mut data_source.project_settings_open,
                project_settings,
            );
        }
        if let Some(project_folder_path) = self.project_folder_path.as_ref() {
            let window = Self::new_window("Content Browser", data_source.input_mode);
            click.content_browser_event = content_browser::draw(
                window,
                context,
                project_folder_path,
                &mut data_source.content_data_source,
                &mut self.thumbnail_cache,
                // data_source.input_mode,
            );
        }
        if let Some(console_cmds) = &data_source.console_cmds {
            let window = Self::new_window("Console Cmds", data_source.input_mode);
            console_cmds_view::draw(
                window,
                context,
                &mut data_source.is_console_cmds_view_open,
                &mut console_cmds.borrow_mut(),
            );
        }

        Self::new_window("Content Property", data_source.input_mode)
            .open(&mut data_source.is_content_item_property_view_open)
            .vscroll(true)
            .hscroll(true)
            .resizable(true)
            .default_size([250.0, 500.0])
            .show(context, |ui| {
                self.content_item_property_view.draw(ui);
            });

        Self::new_window("Object Property", data_source.input_mode)
            .open(&mut data_source.is_object_property_view_open)
            .vscroll(true)
            .hscroll(true)
            .resizable(true)
            .default_size([250.0, 500.0])
            .show(context, |ui| {
                click.object_property_view_event = self.object_property_view.draw(ui);
            });

        Self::new_window("Debug Texture View", data_source.input_mode)
            .open(&mut data_source.is_debug_texture_view_open)
            .vscroll(true)
            .hscroll(true)
            .resizable(true)
            .default_size([500.0, 500.0])
            .show(context, |ui| {
                click.debug_textures_view_event = self.debug_textures_view.draw(ui);
            });

        click
    }

    fn model_hierarchy_window(
        context: &Context,
        data_source: &mut DataSource,
        click: &mut ClickEvent,
    ) {
        Window::new("Model Hierarchy")
            .enabled(data_source.input_mode.is_interact_ui())
            .open(&mut data_source.is_model_hierarchy_open)
            .show(context, |ui| {
                if let Some(model_view_data) = data_source.model_view_data.as_ref() {
                    Self::render_collapsing_header(
                        ui,
                        &model_view_data.mesh_items,
                        &model_view_data.file_path,
                        click,
                    );
                }
            });
    }

    fn render_collapsing_header(
        ui: &mut Ui,
        mesh_items: &[Rc<MeshItem>],
        file_path: &std::path::Path,
        click: &mut ClickEvent,
    ) {
        for mesh_item in mesh_items {
            let id = ui.make_persistent_id(mesh_item.name.clone());
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    if ui.button(mesh_item.name.clone()).clicked() {
                        click.mesh_item = Some(ClickMeshItem {
                            item: mesh_item.clone(),
                            file_path: file_path.to_path_buf(),
                        });
                    }
                })
                .body(|ui| {
                    Self::render_collapsing_header(ui, &mesh_item.childs, file_path, click);
                });
        }
    }

    pub fn new_window(name: &str, input_mode: EInputMode) -> egui::Window<'static> {
        Window::new(name)
            .enabled(input_mode.is_interact_ui())
            .interactable(input_mode.is_interact_ui())
    }
}
