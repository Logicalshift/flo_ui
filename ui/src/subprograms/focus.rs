use super::control_id::*;
use super::region_id::*;
use crate::util::*;
use crate::focus::*;

use flo_scene::*;
use flo_scene::programs::*;
use flo_draw::*;

use futures::prelude::*;
use serde::*;

///
/// Subprogram ID for the default 'focus' subprogram in a scene
///
pub fn subprogram_focus() -> SubProgramId { SubProgramId::called("flo_ui::focus") }

///
/// Requests to the Focus subprogram.
///
/// Focus deals with mapping mouse clicks on a document window to the subprogram responsible for
/// processing them, as well as routing keyboard events to the control that currently has focus.
/// It's a way to divide a window into arbitrarily shaped regions.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Focus {
    /// An event in need of routing
    Event(DrawEvent),

    /// An update from the scene (used to track when subprograms go away)
    Update(SceneUpdate),

    /// Sets which region/control should receive keyboard events
    SetKeyboardFocus(RegionId, ControlId),

    /// Sets which control should receive keyboard focus after the specified control (within a region, which might have several controls)
    ///
    /// This moves the first control so that it's ordered before the second control (new controls are added at the end of the list)
    SetFollowingControl(RegionId, ControlId, ControlId),

    /// Sets which region should receive keyboard focus after reaching the end of the controls in the first region
    ///
    /// This moves the first region so that it's ordered before the second region (new regions are added at the end of the list)
    SetFollowingRegion(RegionId, RegionId),

    /// Move keyboard focus to the next control
    FocusNext,

    /// Move keyboard focus to the preceding control
    FocusPrevious,

    /// Sets which subprogram receives canvas events (events that don't hit any control region)
    SetCanvas(SubProgramId),

    /// Claims a region inside the specified path as belonging to the specified subprogram. The z-index is used to disambiguate requests if more than region matches
    /// Clicks in this region will have 'None' as the control ID
    ClaimRegion { region_id: RegionId, program: SubProgramId, region: Vec<UiPath>, z_index: usize },

    /// Claims a region for a single control within the region for a subprogram. The z-index here is used to disambiguate when multiple regions matches
    ClaimControlRegion { region_id: RegionId, program: SubProgramId, region: Vec<UiPath>, control: ControlId, z_index: usize },

    /// Removes a claim added by ClaimRegion (including any controls within that region)
    RemoveClaim(RegionId),

    /// Removes a claim added by ClaimControlRegion
    RemoveControlClaim(RegionId, ControlId),
}

impl SceneMessage for Focus {
    fn default_target() -> StreamTarget {
        subprogram_focus().into()
    }

    fn initialise(init_context: &impl SceneInitialisationContext) {
        // Set up filters for the focus events/updates
        init_context.connect_programs(StreamSource::Filtered(FilterHandle::for_filter(|scene_updates| scene_updates.map(|update| Focus::Update(update)))), (), StreamId::with_message_type::<SceneUpdate>()).ok();
        init_context.connect_programs(StreamSource::Filtered(FilterHandle::for_filter(|draw_events| draw_events.map(|event| Focus::Event(event)))), (), StreamId::with_message_type::<DrawEvent>()).ok();

        // Create the standard focus subprogram when a message is sent for the first tiem
        init_context.add_subprogram(subprogram_focus(), focus, 20);

        // This is the default target for focus messages to this scene
        init_context.connect_programs((), subprogram_focus(), StreamId::with_message_type::<Focus>()).ok();
    }
}
