use super::button_state::*;
use super::regions::*;
use crate::subprograms::*;
use crate::util::*;

use flo_scene::*;
use flo_draw::*;
use flo_curves::geo::*;
use flo_curves::bezier::path::*;
use flo_curves::bezier::rasterize::*;
use flo_curves::bezier::vectorize::*;

use futures::prelude::*;

use std::collections::{HashMap, HashSet};

///
/// Helps manage the state for the focus subprogram
///
pub (super) struct FocusProgram {
    /// The program that canvas events get sent to (events that aren't for any region)
    pub (super) canvas_program: Option<SubProgramId>,

    /// x-oriented 1D scan space for subprogram regions (or None if this hasn't been calculated)
    pub (super) subprogram_space: Option<Space1D<RegionId>>,

    /// The data for each focus region
    pub (super) region_data: HashMap<RegionId, SubProgramRegion>,

    /// The control that pointer events should be sent to
    pub (super) pointer_target: Option<OutputSink<FocusPointerEvent>>,

    /// The subprogram ID of the active pointer target
    pub (super) pointer_target_program: Option<SubProgramId>,

    /// The control of the active pointer target
    pub (super) pointer_target_control: Option<ControlId>,

    /// State of the mouse buttons
    pub (super) button_state: ButtonState,

    /// The region that currently has keyboard focus
    pub (super) focused_region: Option<RegionId>,

    /// The control within the region that has keyboard focus
    pub (super) focused_control: Option<ControlId>,

    /// Where keyboard events should be sent
    pub (super) focused_event_target: Option<OutputSink<FocusKeyboardEvent>>,

    /// The tab ordering for the controls within each region
    pub (super) tab_ordering: HashMap<RegionId, KeyboardSubProgram>,

    /// The focus order for regions
    pub (super) region_order: Vec<RegionId>,

    /// The bounding box of the window (None if this has not been sent to us)
    pub (super) bounds: Option<(f64, f64)>,

    /// The scale of the window (None if this has not been sent to us)
    pub (super) scale: Option<f64>,

    /// The last control that we found we were hovering over
    pub (super) hover: (Option<SubProgramId>, Option<ControlId>),
}

impl FocusProgram {
    ///
    /// Sets the scale of the window
    ///
    pub async fn set_scale(&mut self, scale: f64, context: &SceneContext) {
        self.scale = Some(scale);
        self.send_to_all(FocusWindowEvent::Scale(scale), &context).await;
    }

    ///
    /// Sets the bounds of the window
    ///
    pub async fn set_bounds(&mut self, width: f64, height: f64, context: &SceneContext) {
        self.bounds = Some((width, height));
        self.send_to_all(FocusWindowEvent::Resize(width, height), &context).await;
    }

    ///
    /// Sends greeting messages to a newly added subprogram
    ///
    pub async fn greet_new_subprogram(&mut self, subprogram_id: SubProgramId, context: &SceneContext) {
        let subprogram = context.send(subprogram_id).ok();

        if let Some(mut subprogram) = subprogram {
            // Send the scale if it's been stored
            if let Some(scale) = self.scale {
                subprogram.send(FocusWindowEvent::Scale(scale)).await.ok();
            }

            // Send the bounds if they've been stored
            if let Some((w, h)) = self.bounds {
                subprogram.send(FocusWindowEvent::Resize(w, h)).await.ok();
            }
        }
    }

    ///
    /// Looks up the event_target SubProgramId for a given region/control pair
    ///
    fn find_event_target(&self, region_id: RegionId, control_id: ControlId) -> Option<SubProgramId> {
        let region = self.region_data.get(&region_id)?;

        // If there's a control with its own event_target, use that
        if let Some(control) = region.controls.iter().find(|c| c.id == control_id) {
            Some(control.event_target)
        } else {
            Some(region.event_target)
        }
    }

    ///
    /// Sets keyboard focus to a specific region/control
    ///
    pub async fn set_keyboard_focus(&mut self, region_id: RegionId, control_id: ControlId, context: &SceneContext) {
        // Unfocus the existing region/control
        if let (Some(old_region_id), Some(old_control_id)) = (self.focused_region, self.focused_control) {
            let old_event_target = self.find_event_target(old_region_id, old_control_id);

            if let Some(old_event_target) = old_event_target {
                if let Ok(mut channel) = context.send(old_event_target) {
                    channel.send(FocusKeyboardEvent::Unfocused(old_control_id)).await.ok();
                }
            }

            self.focused_region  = None;
            self.focused_control = None;
        }

        // Update the focused region/control and inform the relevant event target
        let event_target = self.find_event_target(region_id, control_id);

        if let Some(event_target) = event_target {
            if let Ok(mut channel) = context.send(event_target) {
                self.focused_region  = Some(region_id);
                self.focused_control = Some(control_id);
                channel.send(FocusKeyboardEvent::Focused(control_id)).await.ok();

                self.focused_event_target = Some(channel);
            } else {
                self.focused_event_target = None;
            }
        } else {
            self.focused_event_target = None;
        }
    }

    ///
    /// Sets the following control for keyboard focus within a region (inserting control_id before next_control_id)
    ///
    pub async fn set_following_control(&mut self, region_id: RegionId, control_id: ControlId, next_control_id: ControlId) {
        // Ensure the region is in the ordering
        if !self.region_order.iter().any(|r| r == &region_id) {
            self.region_order.push(region_id);
        }

        let controls_for_region = self.tab_ordering.entry(region_id)
            .or_insert_with(|| KeyboardSubProgram {
                control_order: vec![],
            });

        // Remove the control if it already has an order
        controls_for_region.control_order.retain(|ctrl| ctrl != &control_id);

        // Add next_control_id to the end of the list if it's not there already
        let before_idx = if let Some(idx) = controls_for_region.control_order.iter().position(|ctrl| ctrl == &next_control_id) {
            idx
        } else {
            let idx = controls_for_region.control_order.len();
            controls_for_region.control_order.push(next_control_id);

            idx
        };

        // Insert the control before the 'next' control
        controls_for_region.control_order.insert(before_idx, control_id);
    }

    ///
    /// Sets the following region for keyboard focus (inserting region_id before next_region_id)
    ///
    pub async fn set_following_region(&mut self, region_id: RegionId, next_region_id: RegionId) {
        // Ensure that the control order exists for the regions
        self.tab_ordering.entry(region_id)
            .or_insert_with(|| KeyboardSubProgram {
                control_order: vec![],
            });
        self.tab_ordering.entry(next_region_id)
            .or_insert_with(|| KeyboardSubProgram {
                control_order: vec![],
            });

        // Remove region_id from the existing list
        self.region_order.retain(|r| r != &region_id);

        // Find the index to add region_id before
        let before_idx = if let Some(idx) = self.region_order.iter().position(|r| r == &next_region_id) {
            idx
        } else {
            // Add next_region_id at the end if it doesn't already exist
            let idx = self.region_order.len();
            self.region_order.push(next_region_id);

            idx
        };

        // Insert region_id before next_region_id
        self.region_order.insert(before_idx, region_id);
    }

    ///
    /// Figures out the following region ID in focus order
    ///
    fn next_region(&self, current_region: Option<RegionId>) -> Option<RegionId> {
        let current_region  = current_region?;
        let current_idx     = self.region_order.iter().position(|r| r == &current_region)?;
        let next_idx        = if current_idx+1 >= self.region_order.len() { 0 } else { current_idx+1 };

        Some(self.region_order[next_idx])
    }

    ///
    /// Figures out the following control
    ///
    fn next_control(&self, current_region: Option<RegionId>, current_control: Option<ControlId>) -> Option<(RegionId, ControlId)> {
        let current_region  = current_region?;
        let current_control = current_control?;

        // Get the data for the current region
        let region_data = self.tab_ordering.get(&current_region)?;
        let control_pos = region_data.control_order.iter().position(|ctrl| ctrl == &current_control)?;

        // If this would loop back to the beginning then focus the next region
        if control_pos+1 >= region_data.control_order.len() {
            let next_region_id  = self.next_region(Some(current_region))?;
            let next_region     = self.tab_ordering.get(&next_region_id)?;
            let first_control   = next_region.control_order.iter().copied().next()?;

            Some((next_region_id, first_control))
        } else {
            Some((current_region, region_data.control_order[control_pos+1]))
        }
    }

    ///
    /// Focuses the next control in the list
    ///
    pub async fn focus_next(&mut self, context: &SceneContext) {
        if let Some((next_region, next_control)) = self.next_control(self.focused_region, self.focused_control) {
            self.set_keyboard_focus(next_region, next_control, context).await;
        } else {
            // TODO: focus the first control in the first region
        }
    }

    ///
    /// Figures out the previous region ID in focus order
    ///
    fn previous_region(&self, current_region: Option<RegionId>) -> Option<RegionId> {
        let current_region  = current_region?;
        let current_idx     = self.region_order.iter().position(|r| r == &current_region)?;
        let previous_idx    = if current_idx == 0 { self.region_order.len()-1 } else { current_idx-1 };

        Some(self.region_order[previous_idx])
    }

    ///
    /// Figures out the preceding control
    ///
    fn previous_control(&self, current_region: Option<RegionId>, current_control: Option<ControlId>) -> Option<(RegionId, ControlId)> {
        let current_region  = current_region?;
        let current_control = current_control?;

        // Get the data for the current region
        let region_data = self.tab_ordering.get(&current_region)?;
        let control_pos = region_data.control_order.iter().position(|ctrl| ctrl == &current_control)?;

        if control_pos == 0 {
            // Return the last control of the previous region
            let previous_region_id  = self.previous_region(Some(current_region))?;
            let previous_region     = self.tab_ordering.get(&previous_region_id)?;
            let previous_control    = previous_region.control_order.last()?;

            Some((previous_region_id, *previous_control))
        } else {
            Some((current_region, region_data.control_order[control_pos-1]))
        }
    }

    ///
    /// Focuses the previous control in the list
    ///
    pub async fn focus_previous(&mut self, context: &SceneContext) {
        if let Some((previous_region, previous_control)) = self.previous_control(self.focused_region, self.focused_control) {
            self.set_keyboard_focus(previous_region, previous_control, context).await;
        } else {
            // TODO: focus the last control in the last region
        }
    }

    ///
    /// Sets the program where mouse events go if there's no region defined
    ///
    pub async fn set_canvas(&mut self, canvas_program: SubProgramId) {
        self.canvas_program = Some(canvas_program);
    }

    ///
    /// Marks a region as belonging to a certain control
    ///
    pub async fn claim_region(&mut self, region_id: RegionId, event_target: SubProgramId, region: Vec<UiPath>, control: Option<ControlId>, z_index: usize, context: &SceneContext) {
        // Get the bounds of the region
        let bounds = region.iter()
            .map(|path| path.bounding_box::<Bounds<_>>())
            .reduce(|a, b| a.union_bounds(b))
            .unwrap_or(Bounds::empty());

        // Create a path contour from the region (we use a contour size of 0, 0 as we're not actually scan-converting this path)
        let contour_size    = ContourSize(bounds.max().x().max(0.0) as _, bounds.max().y().max(0.0) as _);
        let region          = PathContour::from_path(region, contour_size);

        // Look up or create the region data
        let region_data = self.region_data.get_mut(&region_id);
        let region_data = if let Some(region_data) = region_data {
            region_data
        } else {
            // Add a new region
            let new_region = SubProgramRegion {
                event_target:   event_target,
                region:         PathContour::from_path::<UiPath>(vec![], contour_size),
                bounds:         bounds.clone(),
                controls:       vec![],
                z_index:        0,
            };
            self.region_data.insert(region_id, new_region);

            // Send the greeting to the subprogram
            self.greet_new_subprogram(event_target, context).await;

            // Fetch the region we just added
            self.region_data.get_mut(&region_id).unwrap()
        };

        if let Some(control) = control {
            // Make sure the bounds includes the region
            region_data.bounds = region_data.bounds.union_bounds(bounds);

            // Add a new control (the control may route events to a different program than the base region)
            region_data.controls.push(SubProgramControl {
                id:             control,
                event_target:   event_target,
                region:         region,
                bounds:         bounds,
                z_index:        z_index,
            })
        } else {
            // Not setting the region for a control: update the base region shape and program
            let combined_bounds = region_data.controls.iter()
                .fold(bounds, |a, b| a.union_bounds(b.bounds));

            region_data.event_target    = event_target;
            region_data.region          = region;
            region_data.bounds          = combined_bounds;
            region_data.z_index         = z_index;
        }

        // Space becomes None (need to recalculate it before we can handle click events)
        self.subprogram_space = None;
    }

    ///
    /// Removes all claims for the specified region
    ///
    pub async fn remove_region_claims(&mut self, region_id: RegionId) {
        self.region_data.remove(&region_id);
        self.subprogram_space = None;
    }

    ///
    /// Removes all claims that belong to the specified program (called when the program stops)
    ///
    pub async fn remove_program_claims(&mut self, program: SubProgramId) {
        // Remove all base regions that belong to this program, plus any controls in other regions
        self.region_data.retain(|_, region| region.event_target != program);
        for region in self.region_data.values_mut() {
            region.controls.retain(|ctrl| ctrl.event_target != program);
        }
        self.subprogram_space = None;
    }

    ///
    /// Removes all keyboard focus tracking for regions belonging to the specified program
    ///
    pub async fn remove_program_focus(&mut self, program: SubProgramId) {
        // Find all regions whose event_target matches this program
        let regions_to_remove = self.region_data.iter()
            .filter(|(_, region)| region.event_target == program)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        for region_id in regions_to_remove {
            self.tab_ordering.remove(&region_id);
            self.region_order.retain(|r| r != &region_id);
        }
    }

    ///
    /// Removes the claim that matches the specified control
    ///
    pub async fn remove_control_claims(&mut self, region_id: RegionId, control: ControlId) {
        if let Some(region_data) = self.region_data.get_mut(&region_id) {
            region_data.controls.retain(|item| item.id != control);
        }
    }

    ///
    /// Sends an event to whichever program/control is focused
    ///
    pub async fn send_to_focus(&mut self, event: FocusKeyboardEvent, context: &SceneContext) {
        let control = self.focused_control;
        let event   = event.with_target(control);

        if let Some(pointer_target_program) = &mut self.pointer_target_program {
            // While performing actions, we try to send to the pointer target instead (if it accepts keyboard events)
            if let Ok(mut pointer_target) = context.send(*pointer_target_program) {
                pointer_target.send(event).await.ok();
            }
        } else if let Some(focus_target) = &mut self.focused_event_target {
            // Send to the keyboard focus if there's no pointer target
            if focus_target.send(event).await.is_err() {
                self.focused_event_target = None;
            }
        }
    }

    ///
    /// Determines the target program at a location in the canvas, filtering out any programs/controls that shouldn't be matched
    ///
    fn pointer_target_filter(&mut self, location_in_canvas: Option<(f64, f64)>, should_match: impl Fn(SubProgramId, Option<ControlId>) -> bool) -> (Option<SubProgramId>, Option<ControlId>) {
        let space       = &mut self.subprogram_space;
        let region_data = &self.region_data;

        // Generate the space if it's not already generated
        let space = if let Some(space) = space {
            space
        } else {
            *space = Some(Space1D::from_data(region_data.iter().map(|(region_id, region)| (region.bounds.min().x()..region.bounds.max().x(), *region_id))));
            space.as_mut().unwrap()
        };

        // Locate the subprogram that the pointer is over
        if let Some((x, y)) = location_in_canvas {
            // Find all of the regions where the point might be inside
            let mut possible_target_programs = space.data_at_point(x)
                .flat_map(|region_id| region_data.get(region_id).map(|region| (region_id, region)))
                .filter(|(_, region)| UiPoint(x, y).in_bounds(&region.bounds))
                .filter(|(_, region)| region.point_is_inside(x, y))
                .collect::<Vec<_>>();

            // Order by z-index if there are multiple possibilities
            if possible_target_programs.len() > 1 {
                possible_target_programs.sort_by_key(|(_, region)| region.z_index);
            }

            // Locate the control in the target region
            let possible_controls = possible_target_programs.into_iter()
                .rev()                          // Because the 'nearest' region is last due to the ordering
                .map(|(region_id, _)| region_id)
                .flat_map(|target_region_id| {
                    let target_region_id    = *target_region_id;
                    let target_region_data  = region_data.get(&target_region_id);

                    target_region_data
                        .into_iter()
                        .flat_map(move |target_region_data| {
                            let event_target = target_region_data.event_target;

                            // Find the control that the point might be inside
                            let mut possible_controls = target_region_data.controls.iter()
                                .filter(|control| UiPoint(x, y).in_bounds(&control.bounds))
                                .filter(|control| control.point_is_inside(x, y))
                                .collect::<Vec<_>>();

                            // Order by z-index if there are multiple possibilities
                            if possible_controls.len() > 1 {
                                possible_controls.sort_by_key(|control| control.z_index);
                            }

                            // If no controls match, then add in the 'base' event target
                            let base = if possible_controls.is_empty() {
                                Some((event_target, None))
                            } else {
                                None
                            };

                            // The highest z-index is the target control
                            possible_controls.into_iter()
                                .rev()
                                .map(move |control| (control.event_target, Some(control.id)))
                                .chain(base)
                        })
                });

            // Apply the filter to the results (which are now a set of possibilities in descending order)
            let mut filtered_controls = possible_controls.filter(|(program_id, control_id)| should_match(*program_id, *control_id));

            // The first item in this iterator is the 'topmost' control that isn't filtered
            if let Some((program_id, control_id)) = filtered_controls.next() {
                (Some(program_id), control_id)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }

    ///
    /// Determines the target program at a location in the canvas
    ///
    #[inline]
    fn pointer_target(&mut self, location_in_canvas: Option<(f64, f64)>) -> (Option<SubProgramId>, Option<ControlId>) {
        self.pointer_target_filter(location_in_canvas, |_, _| true)
    }

    ///
    /// Send a hover message to the pointer target
    ///
    pub async fn send_hover(&mut self, pointer_state: &PointerState) {
        // 'Hovering' only happens when a mouse button is held down
        if self.button_state.num_buttons_down() == 0 {
            self.hover = (None, None);
            return;
        }

        // Locate the subprogram that the pointer is over
        let current_target          = self.pointer_target_program;
        let current_target_control  = self.pointer_target_control;

        let (hover_program, hover_control) = self.pointer_target_filter(pointer_state.location_in_canvas, |program_id, control_id| {
            Some(program_id) != current_target || control_id != current_target_control
        });

        if &self.hover != &(hover_program, hover_control) {
            // Replace the hover control
            self.hover = (hover_program, hover_control);

            // Send a hover event
            if let Some(hover_program) = hover_program {
                self.send_to_pointer_target(FocusPointerEvent::Hover(hover_program, hover_control)).await;
            }
        }
    }

    ///
    /// Send a drop message to the pointer target
    ///
    pub async fn send_drop(&mut self, pointer_state: &PointerState) {
        // Locate the subprogram that the pointer is over
        let current_target          = self.pointer_target_program;
        let current_target_control  = self.pointer_target_control;

        let (drop_program, drop_control) = self.pointer_target_filter(pointer_state.location_in_canvas, |program_id, control_id| {
            Some(program_id) != current_target || control_id != current_target_control
        });

        if let Some(drop_program) = drop_program {
            self.send_to_pointer_target(FocusPointerEvent::Drop(drop_program, drop_control)).await;
        }
    }

    ///
    /// Sets the target of the pointer target, according to the pointer state
    ///
    pub async fn set_pointer_target(&mut self, pointer_state: &PointerState, context: &SceneContext) {
        // Do nothing if the pointer target is locked (usually by a mouse down operation)
        if self.button_state.num_buttons_down() > 0 {
            return;
        }

        // Locate the subprogram that the pointer is over
        let (target_program, target_program_control) = self.pointer_target(pointer_state.location_in_canvas);

        let pointer_target          = &mut self.pointer_target;
        let pointer_target_program  = &mut self.pointer_target_program;
        let pointer_target_control  = &mut self.pointer_target_control;

        // Connect to the program
        if let Some(target_program) = target_program {
            // Over a specific region
            if *pointer_target_program != Some(target_program) {
                // Indicate that we've left the old control (TODO: track individual pointers separately)
                let old_target_control = pointer_target_control.clone();
                if let Some(old_target) = pointer_target {
                    // Leave the control
                    old_target.send(FocusPointerEvent::Pointer(old_target_control, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();

                    if old_target_control.is_some() {
                        // Also leave the subprogram if we were in a control
                        old_target.send(FocusPointerEvent::Pointer(None, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();
                    }
                }

                // Update the pointer target
                *pointer_target = context.send(target_program).ok();

                if let Some(new_target) = pointer_target {
                    // Indicate that we've entered the new program
                    new_target.send(FocusPointerEvent::Pointer(None, PointerAction::Enter, PointerId(0), PointerState::new())).await.ok();
                }

                // No pointer target control at this point
                *pointer_target_control = None;
            }

            *pointer_target_program = Some(target_program);
        } else if let Some(canvas_program) = self.canvas_program {
            // Over the canvas
            if *pointer_target_program != Some(canvas_program) {
                // Leave the old subprogram + control
                let old_target_control = pointer_target_control.clone();
                if let Some(old_target) = pointer_target {
                    // Leave the control
                    old_target.send(FocusPointerEvent::Pointer(old_target_control, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();

                    if old_target_control.is_some() {
                        // Also leave the subprogram if we were in a control
                        old_target.send(FocusPointerEvent::Pointer(None, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();
                    }

                    *pointer_target_control = None;
                }

                *pointer_target = context.send(canvas_program).ok();

                // Enter the canvas
                if let Some(new_target) = pointer_target {
                    // Indicate that we've entered the new program
                    new_target.send(FocusPointerEvent::Pointer(None, PointerAction::Enter, PointerId(0), PointerState::new())).await.ok();
                }
            }

            *pointer_target_program = Some(canvas_program);
        } else {
            // Leave the current control
            let old_target_control = pointer_target_control.clone();
            if let Some(old_target) = pointer_target {
                // Leave the control
                old_target.send(FocusPointerEvent::Pointer(old_target_control, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();

                if old_target_control.is_some() {
                    // Also leave the subprogram if we were in a control
                    old_target.send(FocusPointerEvent::Pointer(None, PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();
                }

                *pointer_target_control = None;
            }

            // No canvas program set
            *pointer_target         = None;
            *pointer_target_program = None;
            *pointer_target_control = None;
        }

        if &target_program_control != &*pointer_target_control {
            if let (Some(old_control), Some(target)) = (&*pointer_target_control, pointer_target.as_mut()) {
                // Leave the old control
                target.send(FocusPointerEvent::Pointer(Some(old_control.clone()), PointerAction::Leave, PointerId(0), PointerState::new())).await.ok();
            }

            *pointer_target_control = target_program_control.clone();

            if let (Some(new_control), Some(target)) = (&*pointer_target_control, pointer_target.as_mut()) {
                // Enter the new control
                target.send(FocusPointerEvent::Pointer(Some(new_control.clone()), PointerAction::Enter, PointerId(0), PointerState::new())).await.ok();
            }
        }
    }

    ///
    /// Sends an event to whichever program/control is the pointer target
    ///
    pub async fn send_to_pointer_target(&mut self, event: FocusPointerEvent) {
        let control = self.pointer_target_control;

        if let Some(pointer_target) = &mut self.pointer_target {
            if pointer_target.send(event.with_target(control)).await.is_err() {
                self.pointer_target = None;
            }
        }
    }

    ///
    /// Sends an event to all registered programs
    ///
    pub async fn send_to_all(&mut self, event: impl Clone + SceneMessage, context: &SceneContext) {
        // Make a list of all the subprograms we know about
        let all_subprograms = self.region_data.values()
            .map(|region| region.event_target)
            .chain(self.region_data.values().flat_map(|region| region.controls.iter().map(|c| c.event_target)))
            .collect::<HashSet<_>>();

        // Send copies of the events to each one
        let mut send_actions = vec![];

        for program in all_subprograms {
            // Send to each program in turn
            if let Ok(mut target) = context.send(program) {
                let event = event.clone();
                send_actions.push(async move { 
                    if target.is_attached() {
                        target.send(event).await.ok(); 
                    }
                });
            }
        }

        future::join_all(send_actions).await;
    }
}
