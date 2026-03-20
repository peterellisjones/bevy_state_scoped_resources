# bevy_state_scoped_resources

State-scoped resource lifecycle management for [Bevy](https://bevyengine.org/).

Bevy's built-in `StateScoped` removes *entities* when exiting a state, but there is no equivalent for *resources*. This crate fills that gap: declare which resources belong to a state, and they are automatically created on enter and cleaned up on exit.

## Quick Start

```rust
use bevy::prelude::*;
use bevy_state_scoped_resources::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState { #[default] Menu, Playing }

#[derive(Resource, Default)]
struct PlayerScore(u32);

#[derive(Resource)]
struct LevelConfig { /* loaded externally */ }

state_scoped_resources!(GameResources for GameState {
    create: [PlayerScore],
    require: [LevelConfig],
});

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .register_state_scoped_resources::<GameResources, GameState>(
            GameState::Playing,
        )
        .run();
}
```

## Create vs Require

The `state_scoped_resources!` macro supports two resource categories:

- **`create`** -- Resources constructed automatically via `FromWorld` when the state is entered. Removed on exit.
- **`require`** -- Resources that must exist during the state but are inserted externally (e.g. by an `OnEnter` system or a loading screen). Asserted on enter (in debug builds), removed on exit.

Both categories are cleaned up when the state exits. On re-entry, `create` resources are freshly constructed again.

## Debug Assertions

In debug builds (`cfg(debug_assertions)`), the crate registers additional systems that:

1. **Post-enter**: assert all `create` and `require` resources exist after `OnEnter` schedules run.
2. **Continuous**: assert all resources still exist every frame while the state is active.
3. **Post-exit**: assert no resources leaked after `OnExit` schedules run.

These assertions catch common bugs like forgetting to insert a `require` resource or accidentally re-inserting a resource during exit.

### `force_assertions` Feature

To enable these assertion systems in release builds:

```toml
[dependencies]
bevy_state_scoped_resources = { version = "0.1", features = ["force_assertions"] }
```

## Single-Resource Helpers

For resources that don't fit neatly into a batch declaration (e.g. optional resources), use the `StateScopedResourceAppExt` trait:

```rust
use bevy::prelude::*;
use bevy_state_scoped_resources::StateScopedResourceAppExt;

# #[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
# enum GameState { #[default] Menu, Playing }
# #[derive(Resource, Default)]
# struct CursorState;

// Remove when exiting the state (no-op if absent):
// app.remove_resource_on_exit::<CursorState, GameState>(GameState::Playing);

// Assert existence after entering (debug builds only):
// app.assert_resource_on_enter::<CursorState, GameState>(GameState::Playing);

// Both combined:
// app.scope_resource_to_state::<CursorState, GameState>(GameState::Playing);
```

## Bevy Compatibility

| bevy_state_scoped_resources | Bevy  |
|-----------------------------|-------|
| 0.1                        | 0.18  |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
