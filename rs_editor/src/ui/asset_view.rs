use crate::data_source::{AssetFile, AssetFolder};
use egui::{Color32, Context, RichText, Ui, Window};
use rs_engine::file_type::EFileType;

#[derive(Debug)]
pub enum EClickItemType {
    Folder(AssetFolder),
    File(AssetFile),
    SingleClickFile(AssetFile),
    CreateTexture(AssetFile),
    Back,
}

pub fn draw(
    context: &Context,
    open: &mut bool,
    asset_folder: Option<&AssetFolder>,
    highlight_file: Option<&AssetFile>,
) -> Option<EClickItemType> {
    let mut click_back: Option<EClickItemType> = None;
    let mut click_asset: Option<EClickItemType> = None;
    Window::new("Asset")
        .open(open)
        .vscroll(true)
        .hscroll(true)
        .resizable(true)
        .default_size([350.0, 150.0])
        .show(context, |ui| {
            ui.set_max_height(250.0);
            ui.set_max_width(500.0);
            if let Some(asset_folder) = &asset_folder {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("Back").color(Color32::WHITE))
                            .clicked()
                        {
                            click_back = Some(EClickItemType::Back);
                        }
                        ui.label(asset_folder.path.to_str().unwrap());
                    });
                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| {
                        click_asset = draw_content(ui, asset_folder, highlight_file);
                    });
                });
            }
            ui.allocate_space(ui.available_size());
        });
    click_asset.or(click_back)
}

enum EAssetItem<'a> {
    AssetFolder(&'a AssetFolder),
    AssetFile(&'a AssetFile),
}

fn draw_content(
    ui: &mut Ui,
    asset_folder: &AssetFolder,
    highlight_file: Option<&AssetFile>,
) -> Option<EClickItemType> {
    let mut total_items: Vec<EAssetItem> = vec![];
    for folder in &asset_folder.folders {
        total_items.push(EAssetItem::AssetFolder(&folder));
    }
    for file in &asset_folder.files {
        total_items.push(EAssetItem::AssetFile(&file));
    }
    let mut iter = total_items.chunks(10);
    let mut click_item: Option<EClickItemType> = None;
    let mut highlight_item: Option<EClickItemType> = None;

    while let Some(row) = iter.next() {
        ui.horizontal_wrapped(|ui| {
            for item in row {
                match item {
                    EAssetItem::AssetFolder(folder) => {
                        let folder = *folder;
                        ui.push_id(folder.name.clone(), |ui| {
                            let response = ui
                                .vertical(|ui| {
                                    ui.set_max_height(50.0);
                                    ui.set_max_width(50.0);
                                    ui.image(egui::include_image!(
                                        "../../../Resource/Editor/folder.svg"
                                    ));
                                    ui.label(folder.name.clone());
                                })
                                .response;
                            let response = response.interact(egui::Sense::click());
                            if response.double_clicked() {
                                click_item = Some(EClickItemType::Folder(folder.clone()));
                            }
                        });
                    }
                    EAssetItem::AssetFile(file) => {
                        let file = *file;
                        ui.push_id(file.name.clone(), |ui| {
                            ui.vertical(|ui| {
                                ui.set_max_height(50.0);
                                ui.set_max_width(50.0);
                                if let Some(highlight_file) = highlight_file {
                                    if highlight_file.path == file.path {
                                        ui.painter().rect_filled(
                                            ui.max_rect(),
                                            0.0,
                                            Color32::LIGHT_BLUE,
                                        );
                                    }
                                }
                                match file.get_file_type() {
                                    EFileType::Fbx | EFileType::Glb | EFileType::Blend => {
                                        ui.image(egui::include_image!(
                                            "../../../Resource/Editor/model.svg"
                                        ));
                                    }
                                    EFileType::Jpeg
                                    | EFileType::Jpg
                                    | EFileType::Png
                                    | EFileType::Exr
                                    | EFileType::Hdr => {
                                        let url = format!("file://{}", file.path.to_str().unwrap());
                                        ui.image(url);
                                    }
                                }
                                let response = ui.button(file.name.clone());
                                if response.clicked() {
                                    highlight_item =
                                        Some(EClickItemType::SingleClickFile(file.clone()));
                                }
                                if response.double_clicked() {
                                    click_item = Some(EClickItemType::File(file.clone()));
                                }
                                match file.get_file_type() {
                                    EFileType::Fbx | EFileType::Glb | EFileType::Blend => {}
                                    EFileType::Jpeg
                                    | EFileType::Jpg
                                    | EFileType::Png
                                    | EFileType::Exr
                                    | EFileType::Hdr => {
                                        response.context_menu(|ui| {
                                            highlight_item =
                                                Some(EClickItemType::SingleClickFile(file.clone()));
                                            if ui.button("Create texture").clicked() {
                                                click_item = Some(EClickItemType::CreateTexture(
                                                    file.clone(),
                                                ));
                                                ui.close_menu();
                                            }
                                        });
                                    }
                                }
                            });
                        });
                    }
                }
            }
        });
    }

    let item = click_item.or(highlight_item);
    item
}
