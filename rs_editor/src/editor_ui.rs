use crate::data_source::{DataSource, MeshItem};
use crate::editor_ui::load::ImageLoader;
use crate::ui::property_view::EValueModifierType;
use crate::ui::top_menu::TopMenu;
use crate::ui::{asset_view, level_view, property_view, textures_view, top_menu};
use egui::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::{path::PathBuf, rc::Rc};

#[derive(Debug)]
pub struct ClickMeshItem {
    pub file_path: PathBuf,
    pub item: Rc<MeshItem>,
}

#[derive(Default, Debug)]
pub struct ClickEvent {
    pub click_node: Option<level_view::EClickEventType>,
    pub mesh_item: Option<ClickMeshItem>,
    pub click_aseet: Option<asset_view::EClickItemType>,
    pub menu_event: Option<top_menu::EClickEventType>,
    pub property_event: HashMap<String, EValueModifierType>,
    pub texture_view_event: Option<textures_view::EClickItemType>,
}

pub struct EditorUI {
    context: Context,
    image_loader: Option<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    svg_loader: Option<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    asset_folder_path: Option<PathBuf>,
    top_menu: TopMenu,
}

impl EditorUI {
    pub fn new(context: Context) -> Self {
        let image_loader_id = "egui_extras::loaders::image_loader::ImageCrateLoader";
        let svg_loader_id = "egui_extras::loaders::svg_loader::SvgLoader";
        let mut image_loader = None;
        let mut svg_loader = None;
        egui_extras::install_image_loaders(&context);
        for item in context.loaders().image.lock().iter() {
            if item.id() == image_loader_id {
                image_loader = Some(item.clone());
            }
            if item.id() == svg_loader_id {
                svg_loader = Some(item.clone());
            }
        }
        Self {
            context,
            image_loader,
            svg_loader,
            asset_folder_path: None,
            top_menu: TopMenu {
                new_project_name: String::new(),
            },
        }
    }

    pub fn set_asset_folder_path(&mut self, asset_folder_path: Option<PathBuf>) {
        self.asset_folder_path = asset_folder_path;
    }

    pub fn build(&mut self, context: &Context, data_source: &mut DataSource) -> ClickEvent {
        let mut click = ClickEvent::default();
        click.menu_event = self.top_menu.draw(context);

        Self::model_hierarchy_window(context, data_source, &mut click);
        if let Some(level) = &data_source.level {
            click.click_node = crate::ui::level_view::draw(
                context,
                &mut data_source.is_level_view_open,
                &level.as_ref().borrow(),
            );
        }
        click.click_aseet = asset_view::draw(
            context,
            &mut data_source.is_asset_folder_open,
            data_source.current_asset_folder.as_ref(),
            data_source.highlight_asset_file.as_ref(),
        );

        if let Some(asset_folder_path) = self.asset_folder_path.as_ref() {
            click.texture_view_event = textures_view::draw(
                context,
                &mut data_source.textures_view_data_source.is_textures_view_open,
                asset_folder_path,
                data_source
                    .textures_view_data_source
                    .texture_folder
                    .as_ref(),
                data_source
                    .textures_view_data_source
                    .highlight_texture_file
                    .as_ref(),
            );
        }
        if let Some(selected_node) = &mut data_source.property_view_data_source.selected_node {
            let mut selected_node = selected_node.as_ref().borrow_mut();
            click.property_event = property_view::draw(
                context,
                &mut data_source.property_view_data_source.is_open,
                Some(&mut selected_node),
            );
        } else {
            property_view::draw(
                context,
                &mut data_source.property_view_data_source.is_open,
                None,
            );
        }

        click
    }

    fn model_hierarchy_window(
        context: &Context,
        data_source: &mut DataSource,
        click: &mut ClickEvent,
    ) {
        Window::new("Model Hierarchy")
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
}
