use app::{app::App, render::RenderContext};
use asset::{handle::Handle, Asset};
use ecs::{system_param::SystemParam, Res, WinnyResource};
use fxhash::FxHashMap;
use std::ops::{Deref, DerefMut};

use crate::texture::{Image, Texture, TextureAtlas};

pub trait RenderAssetApp {
    fn register_render_asset<R: RenderAsset>(&mut self) -> &mut Self;
}

impl RenderAssetApp for App {
    fn register_render_asset<R: RenderAsset>(&mut self) -> &mut Self {
        self.insert_resource(RenderAssets::<R>::default());
        self
    }
}

/// Collection of type R [`RenderAsset`].
#[derive(WinnyResource, Debug)]
pub struct RenderAssets<R: RenderAsset>(pub FxHashMap<Handle<R::Asset>, R>);

impl<R: RenderAsset> Default for RenderAssets<R> {
    fn default() -> Self {
        Self(FxHashMap::default())
    }
}

impl<R: RenderAsset> Deref for RenderAssets<R> {
    type Target = FxHashMap<Handle<R::Asset>, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<R: RenderAsset> DerefMut for RenderAssets<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// [`wgpu`] assets used for rendering.
pub trait RenderAsset: 'static + Send + Sync {
    type Asset: Asset;
    type Params<'w>: SystemParam;

    fn prepare_asset<'w>(asset: &Self::Asset, params: &Self::Params<'w>) -> Self;
}

impl RenderAsset for Texture {
    type Asset = Image;
    type Params<'w> = Res<'w, RenderContext>;

    fn prepare_asset<'w>(asset: &Self::Asset, context: &Self::Params<'w>) -> Self {
        Texture::from_image(&context.device, &context.queue, asset)
    }
}

impl RenderAsset for TextureAtlas {
    type Asset = Image;
    type Params<'w> = Res<'w, RenderContext>;

    fn prepare_asset<'w>(asset: &Self::Asset, context: &Self::Params<'w>) -> Self {
        TextureAtlas::from_image(&context.device, &context.queue, asset).unwrap()
    }
}
