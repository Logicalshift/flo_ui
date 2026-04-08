use super::point_mapping::*;
use crate::subprograms::*;
use crate::util::*;

use flo_binding::*;
use flo_scene::*;
use flo_scene_binding::*;
use flo_draw::canvas::*;
use flo_curves::arc::*;
use flo_curves::line::*;
use flo_curves::bezier::*;
use flo_curves::bezier::path::*;

use futures::prelude::*;
use serde::*;

use std::f64;
use std::collections::*;
use std::sync::*;

///
/// Describes a focus region in a pie program
///
#[derive(Clone, PartialEq)]
pub struct PieFocusRegion {
    /// The path for this region
    pub (super) path: Vec<UiPath>,

    /// The control ID for this region
    pub (super) control: ControlId,

    /// Where messages are sent for this target
    pub (super) target: SubProgramId,

    /// The z-index for this region
    pub (super) z_index: usize,
}

///
/// Update messages for the focus program
///
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PieFocusUpdate {
    UpdateRegions,
    UpdateRadius,
    UpdatePosition,
}

impl SceneMessage for PieFocusUpdate {
}

impl PieFocusRegion {
    ///
    /// Returns the region to use for this control when the 
    ///
    pub fn region_for_position(&self, center: UiPoint, angle: f64) -> Vec<UiPath> {
        // Rotate then translate the position to get the final location
        let rotate      = Transform2D::rotate(angle as _);
        let translate   = Transform2D::translate(center.x() as _, center.y() as _);

        let transform   = translate * rotate;

        // Transform all of the paths
        self.path.iter()
            .map(|path| path.map_points(|point| {
                let (x,y) = transform.transform_point(point.x() as _, point.y() as _);

                UiPoint(x as _, y as _)
            }))
            .collect()
    }

    ///
    /// Creates the claim for this region when the pie is in a certain position
    ///
    pub fn claim(&self, center: UiPoint, angle: f64) -> Focus {
        Focus::ClaimControlRegion { 
            program:    self.target, 
            control:    self.control,
            region:     self.region_for_position(center, angle),
            z_index:    self.z_index,
        }
    }

    ///
    /// Creates the message to remove the claim for this region
    ///
    pub fn remove_claim(&self) -> Focus {
        Focus::RemoveControlClaim(self.target, self.control)
    }
}

///
/// Tells the Focus program where the controls in a pie slice are located
///
pub async fn pie_dialog_focus_program(
    input:              InputStream<PieFocusUpdate>, 
    context:            SceneContext, 
    pie_mapping:        PieDialogPointMapping,
    focus_programs:     impl Into<BindRef<Arc<HashMap<ControlId, PieFocusRegion>>>>,     
    center:             impl Into<BindRef<UiPoint>>, 
    angle:              impl Into<BindRef<f64>>,
    inner_radius:       impl Into<BindRef<f64>>,
    outer_radius:       impl Into<BindRef<f64>>,
    pie_z_index:        usize,
) {
    let Some(our_program_id) = context.current_program_id() else { return; };

    // The active claims for the controls
    let mut claims = HashMap::new();

    // Set up the bindings that this program will track
    let focus_programs  = focus_programs.into();
    let center          = center.into();
    let angle           = angle.into();
    let inner_radius    = inner_radius.into();
    let outer_radius    = outer_radius.into();

    let position        = computed(move || (center.get(), angle.get()));
    let pie_radius      = computed(move || (inner_radius.get(), outer_radius.get()));

    // The paths making up the main 'slice' of the focus program
    let mut slice       = vec![];

    // Always set up by updating the position and regions first
    let mut input       = stream::iter([PieFocusUpdate::UpdateRadius, PieFocusUpdate::UpdatePosition, PieFocusUpdate::UpdateRegions]).chain(input);
    let Ok(mut focus)   = context.send(()) else { return; };

    // Wait for an idle message before starting to update the focus, so things are settled when we process our first messages
    context.wait_for_idle(10).await;

    let mut region_lifetime     = None;
    let mut radius_lifetime     = None;
    let mut position_lifetime   = None;

    while let Some(msg) = input.next().await {
        match msg {
            PieFocusUpdate::UpdateRegions => {
                // Wait for the scene to become idle so we don't process/send too many updates at once
                context.wait_for_idle(10).await;

                // Send the message when this value changes in the future
                region_lifetime = Some(focus_programs.when_changed(NotifySubprogram::send(PieFocusUpdate::UpdateRegions, &context, our_program_id)));

                // We'll send all of the update messages once we're finished processing the focus messages
                let mut focus_messages = vec![];

                // Read the current state of the focus programs
                let focus_programs  = focus_programs.get();
                let (center, angle) = position.get();

                // Add any new claims
                let new_controls = focus_programs
                    .keys()
                    .filter(|control_id| !claims.contains_key(*control_id))
                    .cloned()
                    .collect::<Vec<_>>();

                for control_id in new_controls.into_iter() {
                    let Some(new_claim) = focus_programs.get(&control_id).cloned() else { continue; };

                    focus_messages.push(new_claim.claim(center, angle));
                    claims.insert(control_id, new_claim);
                }

                // Update any changed claims
                let existing_controls = focus_programs
                    .keys()
                    .filter(|control_id| claims.contains_key(*control_id))
                    .cloned()
                    .collect::<Vec<_>>();

                for control_id in existing_controls.into_iter() {
                    let Some(existing_claim)    = claims.get_mut(&control_id) else { continue; };
                    let Some(new_claim)         = focus_programs.get(&control_id) else { continue; };

                    // Do nothing if the claims match
                    if existing_claim.target == new_claim.target && existing_claim.path == new_claim.path && existing_claim.z_index == new_claim.z_index {
                        continue;
                    }

                    // Update the claim
                    focus_messages.push(existing_claim.remove_claim());

                    existing_claim.path     = new_claim.path.clone();
                    existing_claim.control  = new_claim.control;
                    existing_claim.target   = new_claim.target;

                    focus_messages.push(existing_claim.claim(center, angle));
                }

                // Remove any claims that are no longer present
                let removed_controls = claims
                    .keys()
                    .filter(|control_id| !focus_programs.contains_key(*control_id))
                    .cloned()
                    .collect::<Vec<_>>();

                for control_id in removed_controls.into_iter() {
                    let Some(old_claim) = claims.remove(&control_id) else { continue; };

                    focus_messages.push(old_claim.remove_claim());
                }

                // Send the messages
                for msg in focus_messages.into_iter() {
                    focus.send(msg).await.ok();
                }
            },

            PieFocusUpdate::UpdateRadius => {
                // Notify whenever the radius changes
                radius_lifetime = Some(pie_radius.when_changed(NotifySubprogram::send(PieFocusUpdate::UpdateRadius, &context, our_program_id)));

                let (inner_radius, outer_radius)    = pie_radius.get();
                let (center, angle)                 = position.get();

                // Create the slice path
                let inner_arc = Circle::new(UiPoint(0.0, 0.0), inner_radius);
                let inner_arc = inner_arc.arc(-f64::consts::PI/4.0, f64::consts::PI/4.0).to_bezier_curve::<Curve<UiPoint>>();
                let outer_arc = Circle::new(UiPoint(0.0, 0.0), outer_radius);
                let outer_arc = outer_arc.arc(-f64::consts::PI/4.0, f64::consts::PI/4.0).to_bezier_curve::<Curve<UiPoint>>();

                let path = vec![
                    inner_arc.clone(),
                    line_to_bezier(&(inner_arc.end_point(), outer_arc.end_point())),
                    outer_arc.reverse(),
                    line_to_bezier(&(outer_arc.start_point(), inner_arc.start_point())),
                ];
                let path = UiPath::from_curves(&path);

                slice = vec![path];

                // Generate the 'background' slice for this dialog
                let transform = Transform2D::translate(center.x() as _, center.y() as _) * Transform2D::rotate(angle as _);

                let background_slice = Focus::ClaimRegion { 
                    program:    our_program_id, 
                    region:     slice.iter().map(|path| path.map_points(|UiPoint(x, y)| { let (x, y) = transform.transform_point(x as _, y as _); UiPoint(x as _, y as _) })).collect(), 
                    z_index:    pie_z_index,
                };

                focus.send(background_slice).await.ok();
            }

            PieFocusUpdate::UpdatePosition => {
                // Wait for the scene to become idle so we don't process/send too many updates at once
                context.wait_for_idle(10).await;

                // Send this message when the position changes in the future
                position_lifetime = Some(position.when_changed(NotifySubprogram::send(PieFocusUpdate::UpdatePosition, &context, our_program_id)));

                // Fetch the position
                let (center, angle) = position.get();

                // Update all of the existing claims with the new positions
                let focus_messages = claims.values()
                    .map(|claim| claim.claim(center, angle));

                for msg in focus_messages {
                    focus.send(msg).await.ok();
                }

                // Update the 'background' slice for this dialog
                let transform = Transform2D::translate(center.x() as _, center.y() as _) * Transform2D::rotate(angle as _);

                let background_slice = Focus::ClaimRegion { 
                    program:    our_program_id, 
                    region:     slice.iter().map(|path| path.map_points(|UiPoint(x, y)| { let (x, y) = transform.transform_point(x as _, y as _); UiPoint(x as _, y as _) })).collect(), 
                    z_index:    pie_z_index,
                };

                focus.send(background_slice).await.ok();
            },
        }
    }

    // Done listening for events
    drop(region_lifetime);
    drop(radius_lifetime);
    drop(position_lifetime);

    // Release all the claims
    let remove_claims = claims.values().map(|claim| claim.remove_claim());
    for msg in remove_claims {
        focus.send(msg).await.ok();
    }

    focus.send(Focus::RemoveClaim(our_program_id)).await.ok();
}
