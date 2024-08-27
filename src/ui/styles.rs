use bevy::prelude::*;

#[derive(Resource)]
pub struct UIFont(TextStyle);

pub fn init(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(UIFont(get_text_style(&asset_server)));
}

//asset_server.load caches, so should be fine
pub fn get_large_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
        font_size: 64.0,
        color: Color::WHITE,
    }
}

pub fn get_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
        font_size: 32.0,
        color: Color::WHITE,
    }
}

pub fn get_small_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
        font_size: 16.0,
        color: Color::WHITE,
    }
}
