use std::{collections::HashMap, path::Path};

use crate::load_texture_from_file;

struct CachedTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

pub struct TextureCache {
    cache: HashMap<String, CachedTexture>,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_or_load(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        asset_id: &str,
        base_path: &Path,
    ) -> Result<&wgpu::TextureView, String> {
        if !self.cache.contains_key(asset_id) {
            let path = base_path.join(asset_id);
            let (texture, view, width, height) = load_texture_from_file(device, queue, &path)?;
            self.cache.insert(
                asset_id.to_owned(),
                CachedTexture {
                    texture,
                    view,
                    width,
                    height,
                },
            );
        }
        Ok(&self.cache.get(asset_id).unwrap().view)
    }

    pub fn get(&self, asset_id: &str) -> Option<&wgpu::TextureView> {
        self.cache.get(asset_id).map(|c| &c.view)
    }
}
