use bevy::prelude::*;

#[derive(Resource)]
pub struct UIFont(TextStyle);

pub fn init(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(UIFont(get_text_style(&asset_server)));
}

pub fn get_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("fonts/Kenney Mini.ttf"),
        font_size: 32.0,
        color: Color::WHITE,
    }
}