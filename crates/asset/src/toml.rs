use crate::{reader::ByteReader, Asset, AssetApp, AssetLoader, AssetLoaderError};
use app::prelude::*;
use std::{io::Cursor, sync::Arc};
use taplo::dom;

pub struct TomlPlugin;

impl Plugin for TomlPlugin {
    fn build(&mut self, app: &mut App) {
        app.register_asset::<Toml>()
            .register_asset_loader::<Toml>(TomlAssetLoader);
    }
}

struct TomlAssetLoader;

impl AssetLoader for TomlAssetLoader {
    type Asset = Toml;
    type Settings = ();

    fn extensions(&self) -> &'static [&'static str] {
        &["toml"]
    }

    async fn load(
        reader: crate::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _settings: Self::Settings,
        _path: String,
        ext: &str,
    ) -> Result<Self::Asset, crate::AssetLoaderError> {
        if ext != "toml" {
            return Err(crate::AssetLoaderError::UnsupportedFileExtension);
        }

        Ok(Toml::new(reader)?)
    }
}

/// Parsed Toml file asset. Created by the [`asset::AssetServer`].
#[derive(Debug)]
pub struct Toml {
    head: Arc<dom::Node>,
}

impl Asset for Toml {}

unsafe impl Send for Toml {}
unsafe impl Sync for Toml {}

impl Toml {
    pub(crate) fn new(mut reader: ByteReader<Cursor<Vec<u8>>>) -> Result<Self, AssetLoaderError> {
        let parsed = taplo::parser::parse(reader.read_all_to_string()?.as_str());
        if !parsed.errors.is_empty() {
            return Err(AssetLoaderError::SyntaxError);
        }
        let dom = parsed.into_dom();
        if dom.validate().is_err() {
            return Err(AssetLoaderError::SemanticError);
        }

        Ok(Self {
            head: Arc::new(dom),
        })
    }

    pub fn head(&self) -> &dom::Node {
        &self.head
    }
}
