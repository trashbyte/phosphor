//! Global registry types.


use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;

use vulkano::format::{R8G8B8A8Srgb, R16G16B16A16Sfloat};
use vulkano::image::immutable::ImmutableImage;
use vulkano::device::Queue;
use std::io::BufReader;


/// Global texture registry.
pub struct TextureRegistry {
    ldr_textures: HashMap<String, Arc<ImmutableImage<R8G8B8A8Srgb>>>,
    hdr_textures: HashMap<String, Arc<ImmutableImage<R16G16B16A16Sfloat>>>
}


impl TextureRegistry {
    pub fn new() -> TextureRegistry {
        TextureRegistry {
            ldr_textures: HashMap::new(),
            hdr_textures: HashMap::new(),
        }
    }


    /// Loads the textures from disk, and onto the GPU.
    pub fn load(&mut self, queue: Arc<Queue>) {
        let tex_names = [
            String::from("stone"),
            String::from("dirt"),
            String::from("grass"),
            String::from("test_albedo"),
            String::from("test_normal"),
            String::from("white"),
            String::from("black"),
            String::from("grey_50"),
            String::from("gradient"),
            String::from("checker"),
            String::from("BRDF_Lookup_Smith"),
        ];

        for name in tex_names.iter().clone() {
            let (texture, _future) = {
                let mut path_str = String::from("textures/");
                path_str.push_str(&name);
                path_str.push_str(".png");
                let image = image::open(Path::new(&path_str)).unwrap().to_rgba();
                let (w, h) = image.dimensions();
                let image_data = image.into_raw().clone();

                vulkano::image::immutable::ImmutableImage::from_iter(
                    image_data.iter().cloned(),
                    vulkano::image::Dimensions::Dim2d { width: w, height: h },
                    vulkano::format::R8G8B8A8Srgb,
                    queue.clone()).unwrap()
            };
            self.ldr_textures.insert(name.to_string(), texture);
        }

        let hdr_tex_names = [
            String::from("grass_irr"),
            String::from("grass_rad"),
        ];
        for name in hdr_tex_names.iter().clone() {
            let (texture, _future) = {
                let mut path_str = String::from("textures/hdr/");
                path_str.push_str(&name);
                path_str.push_str(".hdr");
                let file = std::fs::File::open(Path::new(&path_str)).unwrap();
                let reader = image::hdr::HDRDecoder::new(BufReader::new(file)).unwrap();
                let meta = reader.metadata();
                let dimensions = vulkano::image::Dimensions::Dim2d {
                    width: meta.width,
                    height: meta.height
                };
                let image_data: Vec<half::f16> = reader.read_image_hdr()
                                                 .unwrap()
                                                 .iter()
                                                 .flat_map(|f| vec![f[0], f[1], f[2], 1.0])
                                                 .map(|f| half::f16::from_f32(f))
                                                 .collect();

                vulkano::image::immutable::ImmutableImage::from_iter(
                    image_data.iter().cloned(),
                    dimensions,
                    vulkano::format::R16G16B16A16Sfloat,
                    queue.clone()).unwrap()
            };
            self.hdr_textures.insert(name.to_string(), texture);
        }
    }


    /// Gets a handle to the texture with the given name, or None if one couldn't be found.
    pub fn get(&self, name: &str) -> Option<Arc<ImmutableImage<R8G8B8A8Srgb>>> {
        match self.ldr_textures.get(name) {
            Some(arc) => Some(arc.clone()),
            None => None
        }
    }

    pub fn get_hdr(&self, name: &str) -> Option<Arc<ImmutableImage<R16G16B16A16Sfloat>>> {
        match self.hdr_textures.get(name) {
            Some(arc) => Some(arc.clone()),
            None => None
        }
    }
}
