use bevy::prelude::*;
use debug::TextStyle;

#[derive(Resource)]
pub struct UIFont(TextStyle);

pub fn init(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(UIFont(get_text_style(&asset_server)));
}

//asset_server.load caches, so should be fine
pub fn get_large_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 64.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub fn get_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 32.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub fn get_small_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 16.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub const TRANSLUCENT_PANEL_BACKGROUND: Color = Color::hsla(272. / 360., 0.15, 0.15, 0.6);
