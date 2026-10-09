//! State-scoped resource lifecycle management for Bevy.
//!
//! Bevy's built-in `StateScoped` removes
//! *entities* when exiting a state, but there is no equivalent for
//! *resources*. This crate fills that gap.
//!
//! # Quick start
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_state_scoped_resources::*;
//!
//! #[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
//! enum GameState { #[default] Menu, Playing }
//!
//! #[derive(Resource, Default)]
//! struct PlayerScore(u32);
//!
//! state_scoped_resources!(GameResources for GameState {
//!     create: [PlayerScore],
//! });
//! ```
//!
//! Full example with app setup:
//!
//! ```rust,ignore
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

#[cfg(any(debug_assertions, feature = "force_assertions"))]
use bevy::state::state::{StateTransition, StateTransitionEvent, StateTransitionSystems};

/// Extension trait for registering [`StateScopedResources`] on an [`App`].
pub trait StateScopedResourcesAppExt {
    /// Register a [`StateScopedResources`] set for a given state value.
    ///
    /// Inserts `create` resources on enter, removes all resources on exit,
    /// and (in debug builds or with `force_assertions`) continuously asserts
    /// that all resources exist while the state is active.
    fn register_state_scoped_resources<M: StateScopedResources, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self;
}

impl StateScopedResourcesAppExt for App {
    fn register_state_scoped_resources<M: StateScopedResources, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self {
        // OnEnter: construct and insert create resources.
        self.add_systems(OnEnter(state.clone()), M::insert_all);

        // OnExit: remove all resources in the set.
        self.add_systems(OnExit(state.clone()), M::remove_all);

        // Assertion systems -- gated on debug_assertions or force_assertions.
        #[cfg(any(debug_assertions, feature = "force_assertions"))]
        {
            // Post-enter: assert all resources exist.
            // ambiguous_with_all: read-only assertions, no data dependencies.
            let state_for_enter = state.clone();
            self.add_systems(
                StateTransition,
                (move |world: &mut World, mut was_active: Local<bool>| {
                    let is_active = world
                        .get_resource::<State<S>>()
                        .is_some_and(|s| *s.get() == state_for_enter);
                    if is_active && !*was_active {
                        M::assert_all_exist(world);
                    }
                    *was_active = is_active;
                })
                .after(StateTransitionSystems::EnterSchedules)
                .ambiguous_with_all(),
            );

            // Continuous: assert all resources still exist every frame.
            // ambiguous_with_all: read-only assertions, no data dependencies.
            let state_for_continuous = state.clone();
            self.add_systems(
                Update,
                (move |world: &World| {
                    M::assert_all_exist(world);
                })
                .run_if(in_state(state_for_continuous))
                .ambiguous_with_all(),
            );

            // Post-exit: assert no resources leaked.
            // ambiguous_with_all: read-only assertions, no data dependencies.
            let state_for_exit = state;
            self.add_systems(
                StateTransition,
                (move |world: &mut World, mut was_active: Local<bool>| {
                    let is_active = world
                        .get_resource::<State<S>>()
                        .is_some_and(|s| *s.get() == state_for_exit);
                    if !is_active && *was_active {
                        M::assert_none_exist(world);
                    }
                    *was_active = is_active;
                })
                .after(StateTransitionSystems::ExitSchedules)
                .before(StateTransitionSystems::EnterSchedules)
                .ambiguous_with_all(),
            );
        }

        self
    }
}

/// Extension trait for scoping individual resources to a state lifetime.
///
/// For resources with a fixed lifecycle (must exist during a state), prefer
/// [`state_scoped_resources!`] and [`StateScopedResourcesAppExt`].
///
/// This trait is useful for **optional resources** that may or may not exist.
pub trait StateScopedResourceAppExt {
    /// Remove this resource when exiting `state`. No-op if already absent.
    fn remove_resource_on_exit<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self;

    /// Assert this resource exists after entering `state`.
    ///
    /// Gated on `debug_assertions` or the `force_assertions` feature.
    fn assert_resource_on_enter<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self;

    /// Convenience: [`remove_resource_on_exit`](Self::remove_resource_on_exit) +
    /// [`assert_resource_on_enter`](Self::assert_resource_on_enter).
    fn scope_resource_to_state<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self;
}

impl StateScopedResourceAppExt for App {
    fn remove_resource_on_exit<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self {
        self.add_systems(OnExit(state), |mut commands: Commands| {
            commands.remove_resource::<R>();
        });
        self
    }

    fn assert_resource_on_enter<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self {
        #[cfg(not(any(debug_assertions, feature = "force_assertions")))]
        let _ = state;
        #[cfg(any(debug_assertions, feature = "force_assertions"))]
        {
            self.add_systems(
                StateTransition,
                (move |mut reader: MessageReader<StateTransitionEvent<S>>,
                       resource: Option<Res<R>>| {
                    for event in reader.read() {
                        if event.entered == event.exited {
                            continue;
                        }
                        if event.entered.as_ref() == Some(&state) {
                            let resource_name = core::any::type_name::<R>();
                            let state_name = core::any::type_name::<S>();
                            assert!(
                                resource.is_some(),
                                "State-scoped resource {resource_name} must exist \
                                 after entering {state_name}::{state:?}",
                            );
                        }
                    }
                })
                .after(StateTransitionSystems::EnterSchedules)
                .ambiguous_with_all(),
            );
        }
        self
    }

    fn scope_resource_to_state<R: Resource, S: States + Clone>(
        &mut self,
        state: S,
    ) -> &mut Self {
        self.remove_resource_on_exit::<R, S>(state.clone())
            .assert_resource_on_enter::<R, S>(state)
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn all_resources_removed_on_exit() {
        let mut app = test_app();
        app.register_state_scoped_resources::<TestResources, GameState>(
            GameState::Playing,
        );
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.insert_resource(LevelConfig(1));
        });

        app.update();
        transition_to(&mut app, GameState::Playing);

        assert!(app.world().contains_resource::<PlayerScore>());
        assert!(app.world().contains_resource::<LevelConfig>());

        transition_to(&mut app, GameState::Menu);

        assert!(
            !app.world().contains_resource::<PlayerScore>(),
            "PlayerScore should be removed after exiting Playing"
        );
        assert!(
            !app.world().contains_resource::<LevelConfig>(),
            "LevelConfig should be removed after exiting Playing"
        );
    }

    #[cfg(any(debug_assertions, feature = "force_assertions"))]
    #[test]
    #[should_panic(expected = "contract violated")]
    fn panics_when_require_resource_missing() {
        let mut app = test_app();
        app.register_state_scoped_resources::<TestResources, GameState>(
            GameState::Playing,
        );
        // Do NOT add OnEnter for LevelConfig.

        app.update();
        transition_to(&mut app, GameState::Playing);
    }

    #[cfg(any(debug_assertions, feature = "force_assertions"))]
    #[test]
    #[should_panic(expected = "contract violated")]
    fn panics_when_resource_leaks_after_exit() {
        let mut app = test_app();
        app.register_state_scoped_resources::<TestResources, GameState>(
            GameState::Playing,
        );
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.insert_resource(LevelConfig(1));
        });
        // Re-insert PlayerScore on exit to simulate a leak.
        app.add_systems(OnExit(GameState::Playing), |mut commands: Commands| {
            commands.insert_resource(PlayerScore(999));
        });

        app.update();
        transition_to(&mut app, GameState::Playing);
        transition_to(&mut app, GameState::Menu);
    }

    #[test]
    fn reentry_creates_fresh_resources() {
        let mut app = test_app();
        app.register_state_scoped_resources::<TestResources, GameState>(
            GameState::Playing,
        );
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.insert_resource(LevelConfig(1));
        });

        app.update();

        // First entry.
        transition_to(&mut app, GameState::Playing);
        assert_eq!(app.world().resource::<PlayerScore>(), &PlayerScore(0));

        // Exit.
        transition_to(&mut app, GameState::Menu);
        assert!(!app.world().contains_resource::<PlayerScore>());

        // Re-enter.
        transition_to(&mut app, GameState::Playing);
        assert_eq!(
            app.world().resource::<PlayerScore>(),
            &PlayerScore(0),
            "PlayerScore should be freshly constructed on re-entry"
        );
    }

    #[test]
    fn type_ids_returns_all_types() {
        let ids = TestResources::type_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&TypeId::of::<PlayerScore>()));
        assert!(ids.contains(&TypeId::of::<LevelConfig>()));
    }

    // Verify create-only and require-only variants compile.
    state_scoped_resources!(CreateOnlyResources for GameState {
        create: [PlayerScore],
    });

    state_scoped_resources!(RequireOnlyResources for GameState {
        require: [LevelConfig],
    });

    #[test]
    fn create_only_type_ids() {
        let ids = CreateOnlyResources::type_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&TypeId::of::<PlayerScore>()));
    }

    #[test]
    fn require_only_type_ids() {
        let ids = RequireOnlyResources::type_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&TypeId::of::<LevelConfig>()));
    }

    // Verify const-generic resources work.
    #[derive(Resource, Debug, PartialEq)]
    struct Pool<const N: u32>(u32);

    impl<const N: u32> FromWorld for Pool<N> {
        fn from_world(_world: &mut World) -> Self {
            Self(N)
        }
    }

    state_scoped_resources!(ConstGenericResources for GameState {
        create: [Pool<0>, Pool<1>],
    });

    #[test]
    fn const_generic_resources_work() {
        let mut app = test_app();
        app.register_state_scoped_resources::<ConstGenericResources, GameState>(
            GameState::Playing,
        );

        app.update();
        transition_to(&mut app, GameState::Playing);

        assert_eq!(app.world().resource::<Pool<0>>(), &Pool(0));
        assert_eq!(app.world().resource::<Pool<1>>(), &Pool(1));
    }

    // --- Single-resource helper tests ---

    #[derive(Resource, Default, Debug, PartialEq)]
    struct CursorState(u32);

    #[test]
    fn remove_resource_on_exit_removes_when_leaving() {
        let mut app = test_app();
        app.remove_resource_on_exit::<CursorState, GameState>(GameState::Playing);

        app.update();
        transition_to(&mut app, GameState::Playing);

        app.world_mut().insert_resource(CursorState(42));
        assert!(app.world().contains_resource::<CursorState>());

        transition_to(&mut app, GameState::Menu);

        assert!(
            !app.world().contains_resource::<CursorState>(),
            "CursorState should be removed on exit"
        );
    }

    #[test]
    fn remove_resource_on_exit_noop_when_absent() {
        let mut app = test_app();
        app.remove_resource_on_exit::<CursorState, GameState>(GameState::Playing);

        app.update();
        transition_to(&mut app, GameState::Playing);
        // Don't insert CursorState.
        transition_to(&mut app, GameState::Menu);
        // Should not panic.
    }

    #[cfg(any(debug_assertions, feature = "force_assertions"))]
    #[test]
    #[should_panic(expected = "State-scoped resource")]
    fn assert_resource_on_enter_panics_when_missing() {
        let mut app = test_app();
        app.assert_resource_on_enter::<CursorState, GameState>(GameState::Playing);

        app.update();
        transition_to(&mut app, GameState::Playing);
    }

    #[test]
    fn assert_resource_on_enter_ok_when_present() {
        let mut app = test_app();
        app.assert_resource_on_enter::<CursorState, GameState>(GameState::Playing);
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.init_resource::<CursorState>();
        });

        app.update();
        transition_to(&mut app, GameState::Playing);
    }

    #[test]
    fn scope_resource_full_lifecycle() {
        let mut app = test_app();
        app.scope_resource_to_state::<CursorState, GameState>(GameState::Playing);
        app.add_systems(OnEnter(GameState::Playing), |mut commands: Commands| {
            commands.init_resource::<CursorState>();
        });

        app.update();

        transition_to(&mut app, GameState::Playing);
        assert!(app.world().contains_resource::<CursorState>());

        transition_to(&mut app, GameState::Menu);
        assert!(!app.world().contains_resource::<CursorState>());

        transition_to(&mut app, GameState::Playing);
        assert!(app.world().contains_resource::<CursorState>());
    }
}
