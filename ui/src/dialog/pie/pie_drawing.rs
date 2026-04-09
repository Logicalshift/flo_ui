use super::pie_animation::*;
use crate::util::*;

use flo_binding::*;
use flo_draw::canvas::*;
use flo_draw::canvas::scenery::*;
use flo_scene::*;
use flo_scene_binding::*;
use flo_curves::arc::*;
use flo_curves::line::*;
use flo_curves::bezier::*;

use futures::prelude::*;
use serde::*;

use std::sync::*;
use std::f64;

///
/// Actions that can result from a binding changing
///
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PieDrawingUpdate {
    UpdateDrawing,
    UpdatePosition,
    UpdateAnimation,
}

impl SceneMessage for PieDrawingUpdate {
}

///
/// Subprogram that deals with drawing a pie dialog
///
pub async fn pie_dialog_drawing_program(
    input:              InputStream<PieDrawingUpdate>, 
    context:            SceneContext, 
    (namespace, layer): (NamespaceId, LayerId), 
    drawing:            impl Into<BindRef<Arc<Vec<Draw>>>>, 
    center:             impl Into<BindRef<UiPoint>>, 
    angle:              impl Into<BindRef<f64>>, 
    open_anim:          impl Into<BindRef<(PieAnimation, f64)>>,
    inner_radius:       impl Into<BindRef<f64>>,
    outer_radius:       impl Into<BindRef<f64>>,
) {
    let Some(our_program_id) = context.current_program_id() else { return; };

    // Convert the bindings
    let drawing      = drawing.into();
    let center       = center.into();
    let angle        = angle.into();
    let open_anim    = open_anim.into();
    let inner_radius = inner_radius.into();
    let outer_radius = outer_radius.into();

    let slice_drawing = computed(move || (drawing.get(), inner_radius.get(), outer_radius.get()));

    // We keep track of the transform we're applying to the layer
    let mut layer_transform     = Transform2D::identity();
    let mut animation_transform = Transform2D::identity();

    // The position consists of both the center and the angle
    let position = computed(move || (center.get(), angle.get()));

    // Connect to the drawing request program
    let Ok(mut drawing_request) = context.send(()) else { return; };

    // Wait for things to settle down before starting to draw the dialog (so we're consistently after any other setup that might be happening)
    context.wait_for_idle(100).await;

    // Input initially updates all three states, before listening for further updates
    let mut input = stream::iter([PieDrawingUpdate::UpdatePosition, PieDrawingUpdate::UpdateDrawing, PieDrawingUpdate::UpdateAnimation]).chain(input);

    // Note: calling 'when_changed' before retrieving the new value is important to avoid a race condition where the change arrives after we've retrieved it
    let mut drawing_lifetime    = None;
    let mut position_lifetime   = None;

    while let Some(update) = input.next().await {
        match update {
            PieDrawingUpdate::UpdateDrawing => {
                // Request a new update when the program changes
                drawing_lifetime = Some(slice_drawing.when_changed(NotifySubprogram::send(PieDrawingUpdate::UpdateDrawing, &context, our_program_id)));

                // Build a request to send to the drawing request program
                let (slice_drawing, inner_radius, outer_radius) = slice_drawing.get();

                let mut drawing = vec![];
                drawing.push_state();

                // Clear out the layer and reset it
                drawing.namespace(namespace);
                drawing.layer(layer);
                drawing.clear_layer();
                drawing.set_layer_transform(layer_transform * animation_transform);

                // Draw the background of the slice
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
                let pie_slice = UiPath::from_curves(&path);

                drawing.new_path();
                drawing.bezier_path(&pie_slice);

                drawing.line_width_pixels(1.0);
                drawing.stroke_color(Color::Rgba(0.3, 0.6, 0.7, 0.9));
                drawing.fill_color(Color::Rgba(0.85, 0.95, 1.0, 0.95));
                drawing.fill();
                drawing.stroke();

                // Draw the contents of the slice
                drawing.extend(slice_drawing.iter().cloned());

                // TODO: draw the title of the slice

                drawing.pop_state();

                // Redraw the slice
                drawing_request.send(DrawingRequest::Draw(Arc::new(drawing))).await.ok();
            },

            PieDrawingUpdate::UpdatePosition => {
                // This fires when the position changes
                position_lifetime = Some(position.when_changed(NotifySubprogram::send(PieDrawingUpdate::UpdatePosition, &context, our_program_id)));

                // Read the position
                let (center, angle) = position.get();

                // Rotate to the angle, then move to the position
                let rotate      = Transform2D::rotate(-angle as _);
                let translate   = Transform2D::translate(center.x() as _, center.y() as _);

                layer_transform = translate * rotate;

                // Update the layer in the drawing
                let mut drawing = vec![];

                drawing.push_state();

                drawing.namespace(namespace);
                drawing.layer(layer);
                drawing.set_layer_transform(layer_transform * animation_transform);

                drawing.pop_state();

                drawing_request.send(DrawingRequest::Draw(Arc::new(drawing))).await.ok();
            },

            PieDrawingUpdate::UpdateAnimation => { 
                // TODO: action depends on the animation that's running
            },
        }
    }

    // Done listening for events
    drop(drawing_lifetime.take());
    drop(position_lifetime.take());
}
