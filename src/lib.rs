//! State-scoped resource lifecycle management for Bevy.
//!
//! Bevy's built-in [`StateScoped`](bevy::prelude::StateScoped) removes
//! *entities* when exiting a state, but there is no equivalent for
//! *resources*. This crate fills that gap.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_state_scoped_resources::*;
//!
//! #[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
//! enum GameState { #[default] Menu, Playing }
//!
//! #[derive(Resource, Default)]
//! struct PlayerScore(u32);
//!
//! #[derive(Resource)]
//! struct LevelConfig { /* loaded externally */ }
//!
//! state_scoped_resources!(GameResources for GameState {
//!     create: [PlayerScore],
//!     require: [LevelConfig],
//! });
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .init_state::<GameState>()
//!         .register_state_scoped_resources::<GameResources, GameState>(
//!             GameState::Playing,
//!         )
//!         .run();
//! }
//! ```

use bevy::prelude::*;
use core::any::TypeId;

/// Contract for a set of resources scoped to a particular state.
///
/// Implemented by the [`state_scoped_resources!`] macro.
/// Do not implement manually.
pub trait StateScopedResources: 'static + Send + Sync {
    /// Construct (via [`FromWorld`]) and insert all `create` resources.
    fn insert_all(world: &mut World);

    /// Remove all `create` and `require` resources.
    fn remove_all(world: &mut World);

    /// Panic if any resource in the set is missing.
    fn assert_all_exist(world: &World);

    /// Panic if any resource in the set still exists.
    fn assert_none_exist(world: &World);

    /// [`TypeId`]s of every resource in the set.
    fn type_ids() -> Vec<TypeId>;
}
