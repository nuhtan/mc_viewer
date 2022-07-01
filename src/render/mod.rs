use std::{
    fs,
    path::{Path, PathBuf}, sync::{Arc, Mutex},
};

use bevy::utils::HashMap;
use image::{open, DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgba, RgbaImage};
use simple_anvil::{block::Block, chunk::Chunk};

mod models;

pub const NON_SOLID: [&str; 11] = [
    "grass",
    "tall_grass",
    "fern",
    "large_fern",
    "potted_fern",
    "sugar_cane",
    "water",
    "air",
    "pointed_dripstone",
    "bubble_column",
    "cave_air",
];

pub fn render_chunk(
    chunk: Chunk,
    region_file_name: String,
    region_path: PathBuf,
    save_name: String,
    texture_cache: Arc<Mutex<HashMap<String, DynamicImage>>>
) -> String {
    let surface_map = chunk.get_heightmap(false).unwrap();
    let ocean_floor = chunk.get_heightmap(true).unwrap();

    let mut chunk_image = RgbaImage::new(256, 256);
    for x in 0..16 {
        for z in 0..16 {
            let texture_cache = texture_cache.clone();
            let y = surface_map[16 * z + x];
            let block = chunk.get_block(x as i32, y, z as i32);
            if block.id == "cave_air" {
                println!(
                    "Cave Air could be: {}",
                    chunk.get_block(x as i32, ocean_floor[16 * z + x], z as i32)
                );
            }

            let mut texture = get_texture(&block, &region_file_name, &region_path, texture_cache.clone());
            let dims = texture.dimensions();
            if dims.0 > 16 || dims.1 > 16 {
                texture = texture.crop(0, 0, 16, 16);
            }

            let mut block_img = texture.into_rgba8();

            merge_colors(block, &chunk, &mut block_img);
            merge_background(
                &mut block_img,
                &chunk,
                x as i32,
                y,
                z as i32,
                &region_file_name,
                &region_path,
                texture_cache
            );

            image::imageops::overlay(
                &mut chunk_image,
                &block_img,
                (x * 16) as i64,
                (z * 16) as i64,
            );
        }
    }
    
    let mut path = format!(
        "{}\\saves\\{}\\{}",
        std::env::current_dir().unwrap().to_str().unwrap(),
        save_name,
        &region_file_name[0..region_file_name.len() - 4]
    );
    fs::create_dir_all(&path).unwrap();
    chunk_image
        .save(&format!(
            "saves\\{}\\{}\\chunk{}.{}.{}.png",
            save_name,
            &region_file_name[0..region_file_name.len() - 4],
            chunk.x,
            chunk.z,
            chunk.get_last_update()
        ))
        .unwrap();

    path.push_str(
        format!(
            "\\chunk{}.{}.{}.png",
            chunk.x,
            chunk.z,
            chunk.get_last_update()
        )
        .as_str(),
    );
    path
}

fn merge_colors(block: Block, chunk: &Chunk, block_img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let color = match block.id.as_str() {
        "grass_block" | "grass" | "tall_grass" | "fern" | "large_fern" | "potted_fern"
        | "sugar_cane" => match &chunk.get_biome(19).as_str()[10..] {
            "badlands" | "wooded_badlands" | "eroded_badlands" => Some(image::Rgb([144, 129, 77])),
            "desert" | "savanna" | "savanna_plateau" | "windswept_savanna" | "nether_wastes"
            | "soul_sand_valley" | "crimson_forest" | "warped_forest" | "basalt_deltas" => {
                Some(image::Rgb([191, 183, 85]))
            }
            "stony_peaks" => Some(image::Rgb([154, 190, 75])),
            "jungle" | "bamboo_jungle" => Some(image::Rgb([89, 201, 60])),
            "sparse_jungle" => Some(image::Rgb([100, 199, 63])),
            "mushroom_fields" => Some(image::Rgb([85, 201, 63])),
            "swamp" => Some(image::Rgb([106, 112, 57])),
            "plains" | "sunflower_plains" | "beach" | "dripstone_caves" => {
                Some(image::Rgb([145, 189, 89]))
            }
            "forest" | "flower_forest" => Some(image::Rgb([121, 192, 90])),
            "dark_forest" => Some(image::Rgb([80, 122, 50])),
            "birch_forest" | "old_growth_birch_forest" => Some(image::Rgb([136, 187, 103])),
            "ocean"
            | "deep_ocean"
            | "warm_ocean"
            | "lukewarm_ocean"
            | "deep_lukewarm_ocean"
            | "cold_ocean"
            | "deep_cold_ocean"
            | "deep_frozen_ocean"
            | "river"
            | "lush_caves"
            | "the_end"
            | "small_end_islands"
            | "end_barrens"
            | "end_midlands"
            | "end_highlands"
            | "the_void" => Some(image::Rgb([142, 185, 113])),
            "meadow" => Some(image::Rgb([131, 187, 109])),
            "old_growth_pine_taiga" => Some(image::Rgb([134, 184, 127])),
            "taiga" | "old_growth_spruce_taiga" => Some(image::Rgb([134, 183, 131])),
            "windswept_hills" | "windswept_gravelly_hills" | "windswept_forest" | "stony_shore" => {
                Some(image::Rgb([138, 182, 137]))
            }
            "snowy_beach" => Some(image::Rgb([131, 181, 147])),
            "snowy__plains" | "ice_spikes" | "snowy_taiga" | "frozen_ocean" | "frozen_river"
            | "grove" | "snowy_slopes" | "frozen_peaks" | "jagged_peaks" => {
                Some(image::Rgb([128, 180, 151]))
            }
            _ => {
                println!("{}: {}", block.id, chunk.get_biome(19));
                None
            },
        },
        "oak_leaves" | "jungle_leaves" | "acacia_leaves" | "dark_oak_leaves" | "vines" => {
            match &chunk.get_biome(19).as_str()[10..] {
                "badlands" | "wooded_badlands" | "eroded_badlands" => {
                    Some(image::Rgb([252, 186, 3]))
                }
                _ => None,
            }
        }
        "water" => match &chunk.get_biome(19).as_str()[10..] {
            "badlands"
            | "bamboo_jungle"
            | "basalt_deltas"
            | "beach"
            | "birch_forest"
            | "crimson_forest"
            | "dark_forest"
            | "deep_dark"
            | "deep_ocean"
            | "desert"
            | "dripstone_caves"
            | "end_barrens"
            | "end_midlands"
            | "eroded_badlands"
            | "flower_forest"
            | "forest"
            | "frozen_peaks"
            | "grove"
            | "ice_spikes"
            | "jagged_peaks"
            | "jungle"
            | "lush_caves"
            | "mushroom_fields"
            | "nether_wastes"
            | "ocean"
            | "old_growth_birch_forest"
            | "old_growth_pine_taiga"
            | "old_growth_spruce_taiga"
            | "plains"
            | "river"
            | "savanna_plateau"
            | "savanna"
            | "small_end_islands"
            | "snowy_plains"
            | "snowy Slopes"
            | "soul_sand_valley"
            | "sparse_jungle"
            | "stony_peaks"
            | "stony Shore"
            | "sunflower_plains"
            | "taiga"
            | "the_end"
            | "the_void"
            | "warped_forest"
            | "windswept_forest"
            | "windswept_gravelly_hills"
            | "windswept_hills"
            | "windswept_savanna"
            | "wooded_badlands" => Some(image::Rgb([63, 118, 228])),
            "cold_ocean" | "deep_cold_ocean" | "snowy_taiga" | "snowy_beach" => {
                Some(image::Rgb([61, 87, 214]))
            }
            "frozen_ocean" | "deep_frozen_ocean" | "frozen_river" => {
                Some(image::Rgb([57, 56, 201]))
            }
            "lukewarm_ocean" | "deep_lukewarm_ocean" => Some(image::Rgb([69, 173, 242])),
            "swamp" => Some(image::Rgb([97, 123, 100])),
            "warm_ocean" => Some(image::Rgb([67, 213, 238])),
            "meadow" => Some(image::Rgb([14, 78, 207])),
            _ => None,
        },
        _ => None,
    };
    match color {
        Some(c) => {
            for dim_x in 0..block_img.dimensions().0 {
                for dim_y in 0..block_img.dimensions().1 {
                    let pixel: &mut Rgba<u8> = block_img.get_pixel_mut(dim_x, dim_y);
                    let percent = pixel.0[0] as f32 / 256 as f32;
                    let channels = pixel.channels_mut();
                    channels[0] = (c.0[0] as f32 * percent) as u8;
                    channels[1] = (c.0[1] as f32 * percent) as u8;
                    channels[2] = (c.0[2] as f32 * percent) as u8;
                }
            }
        }
        None => (),
    }
}

fn merge_background(
    block_img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    chunk: &Chunk,
    x: i32,
    y: i32,
    z: i32,
    region_file_name: &String,
    region_path: &PathBuf,
    texture_cache: Arc<Mutex<HashMap<String, DynamicImage>>>
) {
    let mut y = y - 1;
    let mut below = chunk.get_block(x, y, z);
    while NON_SOLID.contains(&below.id.as_str()) {
        y -= 1;
        below = chunk.get_block(x, y, z);
    }
    let mut background_tex = get_texture(&below, region_file_name, region_path, texture_cache);

    let dims = background_tex.dimensions();
    if dims.0 > 16 || dims.1 > 16 {
        background_tex = background_tex.crop(0, 0, 16, 16);
    }

    let mut background_img = background_tex.into_rgba8();

    merge_colors(below, &chunk, &mut background_img);

    image::imageops::overlay(&mut background_img, block_img, 0, 0);
    *block_img = background_img;
}

fn get_texture(b: &Block, region_file_name: &String, region_path: &PathBuf, texture_cache: Arc<Mutex<HashMap<String, DynamicImage>>>) -> DynamicImage {
    let water = Block::from_name("minecraft:water".into(), b.coords, None);
    let block = if b.id == "bubble_column" { &water } else { &b };
    let mut cache = texture_cache.lock().unwrap();

    let mut tex = if cache.contains_key(&block.id) {
        cache.get(&block.id).unwrap().clone()
    }
     else if Path::new(&format!(
        "./assets/minecraft/textures/block/{}.png",
        block.id
    ))
    .exists()
    {
        let img = open(format!(
            "./assets/minecraft/textures/block/{}.png",
            block.id
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if Path::new(&format!(
        "./assets/minecraft/textures/block/{}_top.png",
        block.id
    ))
    .exists()
    {
        let img = open(format!(
            "./assets/minecraft/textures/block/{}_top.png",
            block.id
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if Path::new(&format!(
        "./assets/minecraft/textures/block/{}_still.png",
        block.id
    ))
    .exists()
    {
        let img = open(format!(
            "./assets/minecraft/textures/block/{}_still.png",
            block.id
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if Path::new(&format!(
        "./assets/minecraft/textures/block/{}.png",
        block.id.split("_").collect::<Vec<&str>>()[0]
    ))
    .exists()
    {
        let img = open(format!(
            "./assets/minecraft/textures/block/{}.png",
            block.id.split("_").collect::<Vec<&str>>()[0]
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if Path::new(&format!(
        "./assets/minecraft/textures/block/{}_down_tip.png",
        block.id
    ))
    .exists()
    {
        let img = open(format!(
            "./assets/minecraft/textures/block/{}_down_tip.png",
            block.id
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if fs::read_dir("./assets/minecraft/textures/block/")
        .unwrap()
        .any(|f| {
            let name = f.unwrap().file_name();
            let sections = name
                .to_str()
                .unwrap()
                .split(block.id.as_str())
                .collect::<Vec<&str>>();
            sections[0] == "" && sections[1].len() == 6
        })
    {
        println!("Found variant block: {}", block.id);
        let mut variants = fs::read_dir("./assets/minecraft/textures/block/")
            .unwrap()
            .map(|f| f.unwrap())
            .filter(|f| {
                let name = f.file_name();
                let sections = name
                    .to_str()
                    .unwrap()
                    .split(block.id.as_str())
                    .collect::<Vec<&str>>();
                sections[0] == "" && sections[1].len() == 6
            })
            .map(|f| f.path())
            .collect::<Vec<PathBuf>>();
        for variant in variants.clone() {
            println!("{}", variant.file_stem().unwrap().to_str().unwrap());
        }
        variants.sort_by(|a, b| {
            // Sorts by the last character of the file stem (name without extension)
            a.file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .chars()
                .next_back()
                .unwrap()
                .to_digit(10)
                .unwrap()
                .partial_cmp(
                    &b.file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .chars()
                        .next_back()
                        .unwrap()
                        .to_digit(10)
                        .unwrap(),
                )
                .unwrap()
        });
        let img = open(format!(
            "./assets/minecraft/textures/block/{}.png",
            variants
                .last()
                .unwrap()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
        )).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else if block.id.contains("fence") {
        let img = models::generate_fence_texture(block, region_file_name, region_path).unwrap();
        cache.insert(block.id.clone(), img.clone());
        img
    } else {
        println!("found block : {}", block.id);
        let mut tex = RgbaImage::new(16, 16);
        for x in 0..16 {
            for z in 0..16 {
                tex.put_pixel(x, z, Rgba([0, 0, 0, 1]))
            }
        }
        DynamicImage::ImageRgba8(tex)
    };

    // If the block texture is greater than 16x16 then we only use a single 16x16 section, this is the case for animated blocks such as water
    let dims = tex.dimensions();
    if dims.0 > 16 || dims.1 > 16 {
        tex = tex.crop(0, 0, 16, 16);
    }
    tex
}
