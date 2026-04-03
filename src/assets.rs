use std::fmt::{Display, Formatter};

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::definition::{StateMachineDefinition, StateMachineDefinitionId, StateMachineLibrary};

#[derive(Asset, Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct StateMachineDefinitionAsset {
    pub definition: StateMachineDefinition,
}

impl StateMachineDefinitionAsset {
    pub fn register(
        &self,
        library: &mut StateMachineLibrary,
    ) -> Result<StateMachineDefinitionId, String> {
        library.register(self.definition.clone())
    }
}

impl From<StateMachineDefinition> for StateMachineDefinitionAsset {
    fn from(definition: StateMachineDefinition) -> Self {
        Self { definition }
    }
}

#[derive(Default, TypePath)]
pub struct StateMachineDefinitionAssetLoader;

#[derive(Debug)]
pub enum StateMachineDefinitionAssetLoaderError {
    Io(std::io::Error),
    Ron(ron::error::SpannedError),
}

impl Display for StateMachineDefinitionAssetLoaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read state-machine asset: {error}"),
            Self::Ron(error) => write!(f, "failed to parse state-machine RON asset: {error}"),
        }
    }
}

impl std::error::Error for StateMachineDefinitionAssetLoaderError {}

impl From<std::io::Error> for StateMachineDefinitionAssetLoaderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ron::error::SpannedError> for StateMachineDefinitionAssetLoaderError {
    fn from(value: ron::error::SpannedError) -> Self {
        Self::Ron(value)
    }
}

impl AssetLoader for StateMachineDefinitionAssetLoader {
    type Asset = StateMachineDefinitionAsset;
    type Settings = ();
    type Error = StateMachineDefinitionAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes::<StateMachineDefinitionAsset>(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["fsm.ron"]
    }
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
