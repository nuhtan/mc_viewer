use std::path::PathBuf;

use image::{open, DynamicImage, GenericImageView, ImageResult, RgbaImage};
use simple_anvil::{block::Block, region::Region};

use crate::render::NON_SOLID;

pub fn generate_fence_texture(
    block: &Block,
    region_file_name: &String,
    region_path: &PathBuf,
) -> ImageResult<DynamicImage> {
    let parts = block.id.split("_fence").collect::<Vec<&str>>();
    let wood_type = parts.first().unwrap();
    let mut base = RgbaImage::new(16, 16);
    println!(
        "Trying to find base wood texture for {}. {}_planks.png?",
        block.id, wood_type
    );
    let wood_tex = open(format!(
        "./assets/minecraft/textures/block/{}_planks.png",
        wood_type
    ))
    .unwrap();

    // Fill center 4x4
    for x in 6..10 {
        for z in 6..10 {
            base.put_pixel(x, z, wood_tex.get_pixel(x, z));
        }
    }

    let (x, y, z) = block.coords.unwrap();
    let block_coords = [
        (x - 1, y, z, 0),
        (x + 1, y, z, 1),
        (x, y, z - 1, 2),
        (x, y, z + 1, 3),
    ];

    let reg_parts = region_file_name.split(".").collect::<Vec<&str>>();
    let reg_coords = (
        reg_parts[1].parse::<i32>().unwrap(),
        reg_parts[2].parse::<i32>().unwrap(),
    );

    // For each adjacent block check if the block is solid, if it is then we draw the non post section of the fence towards the adjacent block
    for coord in block_coords {
        let (x, y, z, dir) = coord;
        let region_folder = region_path.parent().unwrap();
        // println!("{} | {}", region_file_name, region_folder.to_str().unwrap());
        let adj = if x < 0 {
            let region = Region::from_file(format!(
                "{}\\r.{}.{}.mca",
                region_folder.to_str().unwrap(),
                reg_coords.0 - 1,
                reg_coords.1
            ));
            region.get_block(x + 512, y, z).unwrap()
        } else if x >= 512 {
            let region = Region::from_file(format!(
                "{}\\r.{}.{}.mca",
                region_folder.to_str().unwrap(),
                reg_coords.0 + 1,
                reg_coords.1
            ));
            region.get_block(x - 512, y, z).unwrap()
        } else if z < 0 {
            let region = Region::from_file(format!(
                "{}\\r.{}.{}.mca",
                region_folder.to_str().unwrap(),
                reg_coords.0,
                reg_coords.1 - 1
            ));
            region.get_block(x, y, z + 512).unwrap()
        } else if z >= 512 {
            let region = Region::from_file(format!(
                "{}\\r.{}.{}.mca",
                region_folder.to_str().unwrap(),
                reg_coords.0,
                reg_coords.1 + 1
            ));
            region.get_block(x, y, z - 512).unwrap()
        } else {
            // The block is still within the original region
            let region = Region::from_file(format!(
                "{}\\{}",
                region_folder.to_str().unwrap(),
                region_file_name.to_string()
            ));
            region.get_block(x, y, z).unwrap()
        };

        // Connect to all non solid blocks
        if !NON_SOLID.contains(&adj.id.as_str()) {
            match dir {
                0 => {
                    for x in 7..9 {
                        for z in 9..16 {
                            base.put_pixel(x, z, wood_tex.get_pixel(x, z));
                        }
                    }
                }
                1 => {
                    for x in 7..9 {
                        for z in 0..7 {
                            base.put_pixel(x, z, wood_tex.get_pixel(x, z));
                        }
                    }
                }
                2 => {
                    for x in 9..16 {
                        for z in 7..9 {
                            base.put_pixel(x, z, wood_tex.get_pixel(x, z));
                        }
                    }
                }
                3 => {
                    for x in 0..7 {
                        for z in 7..9 {
                            base.put_pixel(x, z, wood_tex.get_pixel(x, z));
                        }
                    }
                }
                _ => unreachable!("This is hardcoded and should be unreachable"),
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(base))
}

pub fn generate_chest_texture() {
    todo!("not yet");
}
