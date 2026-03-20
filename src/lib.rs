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

/// Declares a set of state-scoped resources.
///
/// # Categories
///
/// - **`create`**: Resources constructed via [`FromWorld`] on state enter,
///   removed on exit.
/// - **`require`**: Resources that must exist during the state but are
///   inserted externally (e.g. during loading). Asserted on enter, removed
///   on exit.
///
/// # Example
///
/// ```rust,ignore
/// state_scoped_resources!(pub GameResources for GameState {
///     create: [PlayerScore, PlayerInventory],
///     require: [LevelConfig],
/// });
/// ```
#[macro_export]
macro_rules! state_scoped_resources {
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident for $state_ty:ty {
            $(create: [$($create:ty),* $(,)?],)?
            $(require: [$($require:ty),* $(,)?],)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::StateScopedResources for $name {
            fn insert_all(_world: &mut bevy::prelude::World) {
                $($(
                    let r = <$create as bevy::prelude::FromWorld>::from_world(_world);
                    _world.insert_resource(r);
                )*)?
            }

            fn remove_all(_world: &mut bevy::prelude::World) {
                $($(
                    _world.remove_resource::<$create>();
                )*)?
                $($(
                    _world.remove_resource::<$require>();
                )*)?
            }

            fn assert_all_exist(_world: &bevy::prelude::World) {
                $($(
                    assert!(
                        _world.get_resource::<$create>().is_some(),
                        "{} contract violated: {} missing while in {:?}",
                        stringify!($name),
                        stringify!($create),
                        core::any::type_name::<$state_ty>(),
                    );
                )*)?
                $($(
                    assert!(
                        _world.get_resource::<$require>().is_some(),
                        "{} contract violated: {} missing while in {:?}",
                        stringify!($name),
                        stringify!($require),
                        core::any::type_name::<$state_ty>(),
                    );
                )*)?
            }

            fn assert_none_exist(_world: &bevy::prelude::World) {
                $($(
                    assert!(
                        _world.get_resource::<$create>().is_none(),
                        "{} contract violated: {} leaked after exiting {:?}",
                        stringify!($name),
                        stringify!($create),
                        core::any::type_name::<$state_ty>(),
                    );
                )*)?
                $($(
                    assert!(
                        _world.get_resource::<$require>().is_none(),
                        "{} contract violated: {} leaked after exiting {:?}",
                        stringify!($name),
                        stringify!($require),
                        core::any::type_name::<$state_ty>(),
                    );
                )*)?
            }

            fn type_ids() -> Vec<core::any::TypeId> {
                vec![
                    $($(core::any::TypeId::of::<$create>(),)*)?
                    $($(core::any::TypeId::of::<$require>(),)*)?
                ]
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::*;

    #[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
    enum GameState {
        #[default]
        Menu,
        Playing,
    }

    #[derive(Resource, Debug, PartialEq)]
    struct PlayerScore(u32);

    impl FromWorld for PlayerScore {
        fn from_world(_world: &mut World) -> Self {
            Self(0)
        }
    }

    #[derive(Resource, Debug, PartialEq)]
    struct LevelConfig(u32);

    state_scoped_resources!(TestResources for GameState {
        create: [PlayerScore],
        require: [LevelConfig],
    });

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameState>();
        app
    }

    fn transition_to(app: &mut App, state: GameState) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(state);
        app.update();
    }

    #[test]
    fn create_resources_inserted_on_enter() {
        let mut app = test_app();
        app.register_state_scoped_resources::<TestResources, GameState>(
            GameState::Playing,
        );

        // Insert LevelConfig externally (require resource).
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.insert_resource(LevelConfig(1));
        });

        app.update(); // process default state
        transition_to(&mut app, GameState::Playing);

        assert_eq!(
            app.world().resource::<PlayerScore>(),
            &PlayerScore(0),
            "PlayerScore should be inserted via FromWorld"
        );
        assert_eq!(
            app.world().resource::<LevelConfig>(),
            &LevelConfig(1),
            "LevelConfig should be inserted by OnEnter system"
        );
    }
}
