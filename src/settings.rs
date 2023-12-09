use bevy::{app::AppExit, prelude::*};
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};
use smooth_bevy_cameras::controllers::orbit::OrbitCameraController;

#[derive(Deserialize, Resource, Serialize)]
pub struct GameSettings {
    pub hide_menu_when_game_starts: bool,
    pub next_game: NextGameConfig,
    pub translate_sensitivity: f32,
    pub rotate_sensitivity: f32,
    pub zoom_sensitivity: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            hide_menu_when_game_starts: false,
            next_game: default(),
            translate_sensitivity: 1.0,
            rotate_sensitivity: 0.1,
            zoom_sensitivity: 0.1,
        }
    }
}

impl GameSettings {
    pub fn load(pkv: Res<PkvStore>, mut settings: ResMut<Self>) {
        if let Ok(loaded) = pkv.get::<Self>("settings") {
            *settings = loaded;
        }
    }

    /// Must run in `Last` schedule.
    pub fn save_on_exit_or_request(
        mut exit: EventReader<AppExit>,
        mut request: EventReader<SaveSettings>,
        settings: Res<Self>,
        mut pkv: ResMut<PkvStore>,
    ) {
        if exit.read().count() > 0 || request.read().count() > 0 {
            info!("Saving settings");
            if let Err(e) = pkv.set("settings", &*settings) {
                println!("Failed to save settings: {e}");
            }
        }
    }

    pub fn apply_to_camera(&self, cam: &mut OrbitCameraController) {
        cam.mouse_rotate_sensitivity = Vec2::splat(self.rotate_sensitivity);
        cam.mouse_translate_sensitivity = Vec2::splat(self.translate_sensitivity);
        cam.mouse_wheel_zoom_sensitivity = self.zoom_sensitivity;
        // WASM uses a totally different type of scrolling unit.
        cam.pixels_per_line = 10_000.0 * self.zoom_sensitivity;
    }

    pub fn make_camera(&self) -> OrbitCameraController {
        let mut cam = default();
        self.apply_to_camera(&mut cam);
        cam
    }
}

#[derive(Deserialize, Serialize)]
pub struct NextGameConfig {
    pub squads_per_team: u8,
    pub players_per_squad: u32,
    pub n_balls: u32,
}

impl Default for NextGameConfig {
    fn default() -> Self {
        Self {
            squads_per_team: 2,
            players_per_squad: 100,
            n_balls: 100,
        }
    }
}

#[derive(Event)]
pub struct SaveSettings;
