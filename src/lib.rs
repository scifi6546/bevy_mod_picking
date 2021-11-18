mod events;
mod focus;
mod highlight;
mod mouse;
mod selection;

pub use crate::{
    events::{event_debug_system, mesh_events_system, HoverEvent, PickingEvent, SelectionEvent},
    focus::{mesh_focus, pause_for_picking_blockers, Hover, PickingBlocker},
    highlight::{
        get_initial_mesh_button_material, mesh_highlighting, MeshButtonMaterials, PickableButton,
    },
    mouse::update_pick_source_positions,
    selection::{mesh_selection, NoDeselect, Selection},
};
pub use bevy_mod_raycast::{BoundVol, Primitive3d, RayCastSource};

use bevy::ecs::schedule::ShouldRun;
use bevy::{prelude::*, ui::FocusPolicy};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum PickingSystem {
    BuildRays,
    UpdateRaycast,
    Highlighting,
    Selection,
    PauseForBlockers,
    Focus,
    Events,
}

/// A type alias for the concrete [RayCastMesh](bevy_mod_raycast::RayCastMesh) type used for Picking.
pub type PickableMesh = bevy_mod_raycast::RayCastMesh<PickingRaycastSet>;
/// A type alias for the concrete [RayCastSource](bevy_mod_raycast::RayCastSource) type used for Picking.
pub type PickingCamera = bevy_mod_raycast::RayCastSource<PickingRaycastSet>;

/// This unit struct is used to tag the generic ray casting types `RayCastMesh` and
/// `RayCastSource`. This means that all Picking ray casts are of the same type. Consequently, any
/// meshes or ray sources that are being used by the picking plugin can be used by other ray
/// casting systems because they will have distinct types, e.g.: `RayCastMesh<PickingRaycastSet>`
/// vs. `RayCastMesh<MySuperCoolRaycastingType>`, and as such wil not result in collisions.
pub struct PickingRaycastSet;

pub struct PickingPluginsState {
    pub enable_picking: bool,
    pub enable_highlighting: bool,
    pub enable_interacting: bool,
    pub update_debug_cursor: bool,
    pub print_debug_events: bool,
}

impl Default for PickingPluginsState {
    fn default() -> Self {
        Self {
            enable_picking: true,
            enable_highlighting: true,
            enable_interacting: true,
            update_debug_cursor: true,
            print_debug_events: true,
        }
    }
}

fn simple_criteria(flag: bool) -> ShouldRun {
    if flag {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

pub struct PausedForBlockers(pub(crate) bool);

impl Default for PausedForBlockers {
    fn default() -> Self {
        Self(false)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UpdatePicks {
    EveryFrame(Vec2),
    OnMouseEvent,
}
impl Default for UpdatePicks {
    fn default() -> Self {
        UpdatePicks::EveryFrame(Vec2::ZERO)
    }
}

pub struct DefaultPickingPlugins;
impl Plugin for DefaultPickingPlugins {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(PickingPlugin)
            .add_plugin(InteractablePickingPlugin)
            .add_plugin(HighlightablePickingPlugin);
    }
}
fn enable_picking(state: Res<PickingPluginsState>) -> ShouldRun {
    simple_criteria(state.enable_picking)
}
pub struct PickingPlugin;
impl Plugin for PickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PickingPluginsState>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_run_criteria(enable_picking.system())
                    .with_system(
                        bevy_mod_raycast::update_bound_sphere::<PickingRaycastSet>.system(),
                    )
                    .before(PickingSystem::UpdateRaycast),
            );
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_run_criteria(enable_picking.system())
                .with_system(update_pick_source_positions.system())
                .before(PickingSystem::BuildRays),
        );
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_run_criteria(enable_picking.system())
                .with_system(bevy_mod_raycast::build_rays::<PickingRaycastSet>.system())
                .label(PickingSystem::BuildRays)
                .before(PickingSystem::UpdateRaycast),
        );
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_run_criteria(enable_picking.system())
                .with_system(bevy_mod_raycast::update_raycast::<PickingRaycastSet>.system())
                .label(PickingSystem::UpdateRaycast),
        );
    }
}

fn enable_interating(state: Res<PickingPluginsState>) -> ShouldRun {
    simple_criteria(state.enable_interacting)
}
pub struct InteractablePickingPlugin;
impl Plugin for InteractablePickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PausedForBlockers>()
            .add_event::<PickingEvent>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_system(pause_for_picking_blockers.system())
                    .label(PickingSystem::PauseForBlockers)
                    .after(PickingSystem::UpdateRaycast)
                    .with_run_criteria(enable_interating.system()),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_system(mesh_focus.system())
                    .label(PickingSystem::Focus)
                    .after(PickingSystem::PauseForBlockers)
                    .with_run_criteria(enable_interating.system()),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_system(mesh_selection.system())
                    .label(PickingSystem::Selection)
                    .before(PickingSystem::Events)
                    .after(PickingSystem::Focus)
                    .with_run_criteria(enable_interating.system()),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_system(mesh_events_system.system())
                    .label(PickingSystem::Events)
                    .with_run_criteria(enable_interating.system()),
            );
    }
}

fn enable_highlighting(state: Res<PickingPluginsState>) -> ShouldRun {
    simple_criteria(state.enable_highlighting)
}
pub struct HighlightablePickingPlugin;
impl Plugin for HighlightablePickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<MeshButtonMaterials>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_run_criteria(enable_highlighting.system())
                    .with_system(get_initial_mesh_button_material.system())
                    .after(PickingSystem::UpdateRaycast)
                    .before(PickingSystem::Highlighting),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_run_criteria(enable_highlighting.system())
                    .with_system(mesh_highlighting.system())
                    .label(PickingSystem::Highlighting)
                    .before(PickingSystem::Events),
            );
    }
}

fn update_debug_cursor(state: Res<PickingPluginsState>) -> ShouldRun {
    simple_criteria(state.update_debug_cursor)
}
pub struct DebugCursorPickingPlugin;
impl Plugin for DebugCursorPickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_system(bevy_mod_raycast::update_debug_cursor::<PickingRaycastSet>.system())
                .with_run_criteria(update_debug_cursor.system())
                .after(PickingSystem::UpdateRaycast),
        );
    }
}

fn print_debug_events(state: Res<PickingPluginsState>) -> ShouldRun {
    simple_criteria(state.print_debug_events)
}
pub struct DebugEventsPickingPlugin;
impl Plugin for DebugEventsPickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_system(event_debug_system.system())
                .with_run_criteria(print_debug_events.system())
                .after(PickingSystem::Events),
        );
    }
}

#[derive(Bundle)]
pub struct PickingCameraBundle {
    pub source: PickingCamera,
    pub update: UpdatePicks,
}

impl Default for PickingCameraBundle {
    fn default() -> Self {
        PickingCameraBundle {
            source: PickingCamera::new(),
            update: UpdatePicks::default(),
        }
    }
}

#[derive(Bundle, Default)]
pub struct PickableBundle {
    pub pickable_mesh: PickableMesh,
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy,
    pub pickable_button: PickableButton,
    pub selection: Selection,
    pub hover: Hover,
}
