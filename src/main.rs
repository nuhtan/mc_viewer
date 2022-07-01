#![windows_subsystem = "windows"]

use std::{
    fs::{self, DirEntry},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bevy::{
    asset::AssetServerSettings,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::camera::Camera2d,
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};

use bevy_egui::{egui, EguiContext, EguiPlugin};
use futures_lite::future;
use render::render_chunk;
use simple_anvil::region::Region;

mod render;

#[derive(Clone, PartialEq)]
enum Zoom {
    One,
    Two,
    Three,
    Four,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom::One
    }
}

#[derive(Default, Clone)]
struct UIState {
    save_name: String,
    save_path: String,
    loading: bool,
    zoom: Zoom,
    rendering_count: u32,
}

impl UIState {
    pub fn zoom_in(&mut self) -> bool {
        let end = self.zoom != Zoom::One;
        self.zoom = match self.zoom {
            Zoom::One => Zoom::One,
            Zoom::Two => Zoom::One,
            Zoom::Three => Zoom::Two,
            Zoom::Four => Zoom::Three,
        };
        end
    }

    pub fn zoom_out(&mut self) -> bool {
        let end = self.zoom != Zoom::Four;
        self.zoom = match self.zoom {
            Zoom::One => Zoom::Two,
            Zoom::Two => Zoom::Three,
            Zoom::Three => Zoom::Four,
            Zoom::Four => Zoom::Four,
        };
        end
    }

    pub fn zoom_enumerated(&self) -> u32 {
        match self.zoom {
            Zoom::One => 1,
            Zoom::Two => 2,
            Zoom::Three => 4,
            Zoom::Four => 8,
        }
    }
}

fn main() {
    App::new()
        .insert_resource(AssetServerSettings {
            asset_folder: "./saves".to_string(),
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .init_resource::<UIState>()
        .add_startup_system(setup)
        .add_system(egui)
        .add_system(grab_mouse)
        .add_system(drag_folder)
        .add_system(handle_images_finished)
        .add_system(zoom)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn egui(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: ResMut<UIState>,
    commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
    transforms: ParamSet<(Query<&mut Transform, With<Camera2d>>, Query<&Transform>)>,
    windows: Res<Windows>,
) {
    let mut load = false;
    let mut all = false;
    egui::Window::new("Please Input World Directory").show(egui_context.ctx_mut(), |ui| {
        ui.text_edit_singleline(&mut ui_state.save_path);
        load = ui.button("Render Viewport").clicked();
        all = ui.button("Render All Chunks").clicked();
        ui.label(format!(
            "Currently rendering: {} chunks",
            ui_state.rendering_count
        ));
    });

    if load {
        ui_state.loading = true;
        determine_chunks(commands, thread_pool, transforms, windows, ui_state);
    } else if all {
        ui_state.loading = true;
        render_all(commands, thread_pool, ui_state);
    }
}

fn render_all(
    mut commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
    mut ui_state: ResMut<UIState>,
) {
    let mut path = ui_state.save_path.clone();
    path.push_str("\\region");
    let dir = fs::read_dir(path).unwrap();
    let regions = dir.map(|f| f.as_ref().unwrap().path()).collect::<Vec<PathBuf>>();
    let texture_cache = Arc::new(Mutex::new(HashMap::new()));
    
    for region in regions {
        let reg = Region::from_file(region.clone().to_str().unwrap().into());
        for x in 0..32 {
            for z in 0..32 {
                let region_file_name = reg.filename.clone();
                let region_path = region.clone();
                let chunk = reg.get_chunk(x, z);
                let cache = texture_cache.clone();
                let s_name = ui_state.save_name.clone();
                match chunk {
                    Some(c) => {
                        if c.get_status() == "full" {
                            let task = thread_pool.spawn(async move {
                                Some(render_chunk(c, region_file_name, region_path, s_name, cache))
                            });
                            commands.spawn().insert(task);
                            ui_state.rendering_count += 1;
                        }
                    },
                    None => (),
                }
                
            }
        }
    }
}

fn determine_chunks(
    mut commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
    mut transforms: ParamSet<(Query<&mut Transform, With<Camera2d>>, Query<&Transform>)>,
    windows: Res<Windows>,
    mut ui_state: ResMut<UIState>,
) {
    for transform in transforms.p0().iter() {
        let loc = transform.translation;
        let window = windows.get_primary().unwrap();
        let (window_width, window_height) = (window.width(), window.height());

        let chunks_width =
            (window_width / (16.0 * 16.0) * ui_state.zoom_enumerated() as f32).ceil();
        let chunks_height =
            (window_height / (16.0 * 16.0) * ui_state.zoom_enumerated() as f32).ceil();
        let loc_chunks = (loc.x / (16.0 * 16.0), -loc.y / (16.0 * 16.0));
        let mut chunks = Vec::new();
        for x in (loc_chunks.0 - (chunks_width / 2.0)) as i32 - (ui_state.zoom_enumerated() as i32 / 2)
            ..(loc_chunks.0 + (chunks_width / 2.0)) as i32 + (ui_state.zoom_enumerated() as i32 / 2)
        {
            for y in (loc_chunks.1 - (chunks_height / 2.0)) as i32
                - (ui_state.zoom_enumerated() as i32 / 2)
                ..(loc_chunks.1 + (chunks_height / 2.0)) as i32 + (ui_state.zoom_enumerated() as i32 / 2)
            {
                chunks.push((x, y));
            }
        }
        ui_state.rendering_count += chunks.len() as u32;
        let texture_cache = Arc::new(Mutex::new(HashMap::new()));
        for chunk_coords in chunks {
            let mut path = ui_state.save_path.clone();
            let mut save_path = std::env::current_dir().unwrap();
            save_path.push("saves");
            save_path.push(format!("{}", ui_state.save_name));
            let s_name = ui_state.save_name.clone();
            let cache = texture_cache.clone();
            let task = thread_pool.spawn(async move {
                let region_coords = (
                    (chunk_coords.0 as f64 / 32 as f64).floor(),
                    (chunk_coords.1 as f64 / 32 as f64).floor(),
                );
                path.push_str("\\region");
                let dir = fs::read_dir(path).unwrap();
                match dir
                    .filter(|f| {
                        f.as_ref().unwrap().path().file_name().unwrap()
                            == format!("r.{}.{}.mca", region_coords.0, region_coords.1).as_str()
                    })
                    .map(|f| f.as_ref().unwrap().path())
                    .collect::<Vec<PathBuf>>()
                    .first()
                {
                    Some(region_path) => {
                        let region = Region::from_file(region_path.to_str().unwrap().into());
                        match region.get_chunk(
                            ((32 + chunk_coords.0) % 32) as u32,
                            ((32 + chunk_coords.1) % 32) as u32,
                        ) {
                            Some(chunk) => {
                                if chunk.get_status() == "full" {
                                    save_path.push(
                                        &format!("r.{}.{}", region_coords.0, region_coords.1)
                                            .to_string(),
                                    );
                                    if save_path.exists() {
                                        let rendered_dir = fs::read_dir(save_path).unwrap();
                                        let rendered_chunk = rendered_dir
                                            .filter(|f| {
                                                f.as_ref()
                                                    .unwrap()
                                                    .file_name()
                                                    .to_str()
                                                    .unwrap()
                                                    .contains(
                                                        format!(
                                                            "chunk{}.{}",
                                                            ((32 + chunk_coords.0) % 32),
                                                            ((32 + chunk_coords.1) % 32)
                                                        )
                                                        .as_str(),
                                                    )
                                            })
                                            .map(|f| f.unwrap())
                                            .collect::<Vec<DirEntry>>();
                                        if rendered_chunk.len() > 0 {
                                            // Exists a rendered image
                                            if rendered_chunk
                                                .first()
                                                .unwrap()
                                                .file_name()
                                                .to_str()
                                                .unwrap()
                                                .split(".")
                                                .collect::<Vec<&str>>()
                                                .get(2)
                                                .unwrap()
                                                .to_string()
                                                .parse::<i64>()
                                                .unwrap()
                                                >= *chunk.get_last_update()
                                            {
                                                let content = rendered_chunk.first();
                                                let path = match content {
                                                    Some(entry) => match entry.path().to_str() {
                                                        Some(entry_path) => entry_path.to_string(),
                                                        None => todo!(),
                                                    },
                                                    None => todo!(),
                                                };
                                                return Some(path);
                                            }
                                        }
                                    }

                                    Some(render_chunk(
                                        chunk,
                                        region.filename,
                                        region_path.to_path_buf(),
                                        s_name,
                                        cache,
                                    ))
                                } else {
                                    None // Chunk not fully rendered
                                }
                            }
                            None => None, // Chunk does not exist
                        }
                    }
                    None => None, // Region file does not exist
                }
            });
            commands.spawn().insert(task);
        }
        // println!("{:?} {:?}", width_range, height_range);
    }
}

fn grab_mouse(
    mut windows: ResMut<Windows>,
    mouse_button: Res<Input<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut cameras: Query<(&mut Transform, With<Camera2d>)>,
    ui_state: ResMut<UIState>,
) {
    let window = windows.get_primary_mut().unwrap();
    if mouse_button.just_pressed(MouseButton::Left) {
        window.set_cursor_visibility(false);
    }

    if mouse_button.just_released(MouseButton::Left) {
        window.set_cursor_visibility(true);
        // determine_chunks(commands, thread_pool, transforms, windowsPass, ui_state);
    }

    if mouse_button.pressed(MouseButton::Left) {
        for event in mouse_motion.iter() {
            let delta = event.delta;
            for (mut transform, _) in cameras.iter_mut() {
                transform.translation = Vec3::new(
                    transform.translation.x - delta.x * ui_state.zoom_enumerated() as f32,
                    transform.translation.y + delta.y * ui_state.zoom_enumerated() as f32,
                    transform.translation.z,
                );
            }
        }
    }
}

fn zoom(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut ui_state: ResMut<UIState>,
    mut projections: Query<(&mut OrthographicProjection, With<Camera2d>)>,
) {
    for event in mouse_wheel_events.iter() {
        let zoom = if event.y == 1.0 {
            ui_state.zoom_in()
        } else if event.y == -1.0 {
            ui_state.zoom_out()
        } else {
            false
        };
        if zoom {
            for (mut projection, _) in projections.iter_mut() {
                let zoom_scale = ui_state.zoom_enumerated() as f32;
                projection.scale = zoom_scale;
            }
        }
    }
}

fn handle_images_finished(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut Task<Option<String>>)>,
    asset_server: Res<AssetServer>,
    mut ui_state: ResMut<UIState>,
) {
    for (entity, mut task) in transform_tasks.iter_mut() {
        if let Some(path) = future::block_on(futures_lite::future::poll_once(&mut *task)) {
            match path {
                Some(path_str) => {
                    let parts = path_str
                        .split("\\")
                        .collect::<Vec<&str>>()
                        .last()
                        .unwrap()
                        .split("chunk")
                        .collect::<Vec<&str>>()
                        .last()
                        .unwrap()
                        .split(".")
                        .collect::<Vec<&str>>();
                    let region = path_str.split("\\").collect::<Vec<&str>>();
                    let region_parts = region[region.len() - 2].split(".").collect::<Vec<&str>>();
                    let region_x = region_parts[1].parse::<f32>().unwrap();
                    let region_z = region_parts[2].parse::<f32>().unwrap();
                    let x = parts[0].parse::<f32>().unwrap();
                    let z = parts[1].parse::<f32>().unwrap();
                    commands.spawn_bundle(SpriteBundle {
                        texture: asset_server
                            .load(&path_str.split("saves").collect::<Vec<&str>>()[1][1..]),

                        transform: Transform::from_xyz(
                            x * 256.0 + 8192.0 * region_x + 128.0,
                            (z * 256.0 + 8192.0 * region_z) * -1.0 - 128.0,
                            1.0,
                        ),
                        ..default()
                    });
                }
                None => (), //println!("Unavailable chunk requested"),
            }
            ui_state.rendering_count -= 1;
            commands.entity(entity).remove::<Task<Option<String>>>();
        }
    }
}

fn drag_folder(mut events: EventReader<FileDragAndDrop>, mut ui_state: ResMut<UIState>) {
    for event in events.iter() {
        match event {
            // Only care about dropped files
            FileDragAndDrop::DroppedFile { id: _, path_buf } => {
                // Only care about directories
                if path_buf.is_dir() {
                    ui_state.save_name =
                        path_buf.file_name().unwrap().to_str().unwrap().to_string();
                    // Make sure directory contains a region folder
                    let region = path_buf.join("region");
                    if region.exists() {
                        let mut region_dir = fs::read_dir(region).unwrap();
                        if region_dir
                            .any(|f| f.as_ref().unwrap().path().extension().unwrap() == "mca")
                        {
                            ui_state.save_path = path_buf.to_str().unwrap().into();
                        }
                    }
                }
            }
            // Hover and Cancel dragged files
            _ => (),
        }
    }
}
