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
