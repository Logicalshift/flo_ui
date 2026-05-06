use super::drawing_binding::*;
use super::pie_animation::*;
use super::pie_drawing::*;
use super::pie_focus::*;
use super::point_mapping::*;
use crate::subprograms::*;
use crate::util::*;

use flo_binding::*;
use flo_scene::*;
use flo_scene::programs::*;
use flo_scene_binding::*;
use flo_draw::canvas::*;
use flo_curves::*;

use futures::prelude::*;
use futures::channel::mpsc;

use std::f64;
use std::collections::*;
use std::sync::*;

///
/// Configuration and a way to run pie dialogs
///
#[derive(Clone, Debug)]
pub struct PieDialogProgram {
    /// The animation used to show and hide this dialog
    animation:      PieAnimation,

    /// Where the center of the pie is located
    center:         UiPoint,

    /// The inner radius of the pie slice
    inner_radius:   f64,

    /// The outer radius of the pie slice
    outer_radius:   f64,

    /// The angle of the slice, in radians
    angle:          f64,

    /// The 'width' of the slice in pixels (number of pixels from the flat coordinate scheme that covers the whole range)
    width:          f64,

    /// The title displayed on the outside of the slice
    title:          String,

    /// If true, there's a '...' icon for configuring more details for this program
    has_detail:     bool,

    /// The namespace where this pie is rendered
    namespace:      NamespaceId,

    /// The layer where this pie slice is rendered
    layer:          LayerId,

    /// The z-index for this dialog (as supplied to Focus)
    z_index:        usize,
}

impl Default for PieDialogProgram {
    fn default() -> Self {
        PieDialogProgram {
            namespace:      NamespaceId::new(),
            center:         UiPoint(0.0, 0.0),
            layer:          LayerId(0),
            animation:      PieAnimation::ExpandFanOut,
            inner_radius:   32.0,
            outer_radius:   100.0,
            angle:          f64::consts::PI/2.0,
            width:          100.0,
            title:          String::default(),
            has_detail:     false,
            z_index:        500,
        }
    }
}

// Config

impl PieDialogProgram {
    ///
    /// Creates a new pie dialog with a layer and a namespace
    ///
    pub fn new(center: impl Coordinate + Coordinate2D, angle_degrees: f64, namespace: NamespaceId, layer: LayerId) -> Self {
        Self::default()
            .with_center(center)
            .with_angle(angle_degrees)
            .with_namespace(namespace)
            .with_layer(layer)
    }

    ///
    /// Sets the center position of the pie
    ///
    #[inline]
    pub fn with_center(mut self, center: impl Coordinate + Coordinate2D) -> Self {
        self.center = UiPoint::from_components(&[center.x(), center.y()]);
        self
    }

    ///
    /// Sets the namespace to use for rendering the pie slice
    ///
    #[inline]
    pub fn with_namespace(mut self, new_namespace: NamespaceId) -> Self {
        self.namespace = new_namespace;
        self
    }

    ///
    /// Sets the layer to use for rendering the pie slice
    ///
    #[inline]
    pub fn with_layer(mut self, new_layer: LayerId) -> Self {
        self.layer = new_layer;
        self
    }

    ///
    /// Sets the animation for the program
    ///
    #[inline]
    pub fn with_animation(mut self, new_animation: PieAnimation) -> Self {
        self.animation = new_animation;
        self
    }

    ///
    /// Sets the inner radius of the pie slice
    ///
    #[inline]
    pub fn with_inner_radius(mut self, new_radius: f64) -> Self {
        self.inner_radius = new_radius;
        self
    }

    ///
    /// Sets the outer radius of the pie slice
    ///
    #[inline]
    pub fn with_outer_radius(mut self, new_radius: f64) -> Self {
        self.outer_radius = new_radius;
        self
    }

    ///
    /// Sets the angle of the slice (specified in degrees, converted to radians)
    ///
    #[inline]
    pub fn with_angle(mut self, new_angle_degrees: f64) -> Self {
        self.angle = new_angle_degrees.to_radians();
        self
    }

    ///
    /// Sets the 'width' of the slice in pixels (number of pixels from the flat coordinate scheme that covers the whole range)
    ///
    #[inline]
    pub fn with_width(mut self, new_width: f64) -> Self {
        self.width = new_width;
        self
    }

    ///
    /// Sets the title displayed on the outside of the slice
    ///
    #[inline]
    pub fn with_title(mut self, new_title: impl Into<String>) -> Self {
        self.title = new_title.into();
        self
    }

    ///
    /// Sets whether there's a '...' icon for configuring more details for this program
    ///
    #[inline]
    pub fn with_has_detail(mut self, new_has_detail: bool) -> Self {
        self.has_detail = new_has_detail;
        self
    }

    ///
    /// The Focus z-index for this dialog (used when the dialog overlaps other controls)
    ///
    #[inline]
    pub fn with_z_index(mut self, new_z_index: usize) -> Self {
        self.z_index = new_z_index;
        self
    }

    ///
    /// Retrieves the mapping type for this program (which can be used to find how points map onto the pie layer)
    ///
    #[inline]
    fn point_mapping(&self) -> PieDialogPointMapping {
        PieDialogPointMapping { 
            inner_radius:   self.inner_radius,
            outer_radius:   self.outer_radius,
        }
    }

    ///
    /// The width of the pie slice coordinate system (x goes from -width/2 .. width/2)
    ///
    pub fn width(&self) -> f64 {
        let width = (self.outer_radius + self.inner_radius)*f64::consts::PI/4.0;

        width
    }

    ///
    /// The height of the pie slice coordinate system (y goes from 0 .. height)
    ///
    pub fn height(&self) -> f64 {
        let height = self.outer_radius - self.inner_radius;

        height
    }
}

// Execution

impl PieDialogProgram {
    ///
    /// Runs the subprogram for this pie dialog
    ///
    pub fn run(self, input: InputStream<PieDialog>, context: SceneContext) -> impl 'static + Send + Future<Output=()> {
        async move {
            let Some(our_program_id) = context.current_program_id() else { return };

            // Namespace and layer that we'll be drawing on
            let namespace   = self.namespace;
            let layer       = self.layer;

            // Original angle and center (used when the position changes)
            let original_center = self.center;
            let original_angle  = self.angle;

            // Binding for the instructions used to redraw the pie slice (coordinates transformed)
            let draw_binding    = bind(Arc::<Vec<Draw>>::new(vec![]));
            let focus_claims    = bind(Arc::new(HashMap::new()));

            // Current position is used to calculate the layer transform
            let center      = bind(original_center);
            let angle       = bind(original_angle);

            // Title is rendered at the outer radius
            let inner_radius = bind(self.inner_radius);
            let outer_radius = bind(self.outer_radius);
            let title        = bind(self.title.clone());

            // Animation status
            let animation = self.animation;
            let animation = match self.animation {
                PieAnimation::None  => BindRef::from(computed(move || (animation, 1.0))),
                _                   => {
                    let anim_pos = animate_binding(AnimationDescription::ease_out(20.0), &context);
                    anim_pos.start();

                    BindRef::from(computed(move || (animation, anim_pos.get().min(0.05))))
                },
            };

            // Stream for processing the draw instructions
            let (send_drawing, recv_drawing) = mpsc::channel::<Draw>(1000);

            let with_text_layout    = drawing_with_laid_out_text(recv_drawing);
            let with_glyph_paths    = drawing_with_text_as_paths(with_text_layout);

            // Tell SceneControl to create a subprogram to update the drawing binding whenever the redrawn paths is changed
            let update_draw_binding = draw_binding.clone();
            let point_mapping       = self.point_mapping();

            context.send_message(SceneControl::start_child_program(SubProgramId::new(), our_program_id, move |input, context|
                pie_dialog_drawing_binding_program(input, context, update_draw_binding, point_mapping, with_glyph_paths), 1)).await.ok();

            // Tell SceneControl to create a subprogram to perform the actual rendering of the slice
            let draw_center       = center.clone();
            let draw_angle        = angle.clone();
            let draw_animation    = animation.clone();
            let draw_inner_radius = inner_radius.clone();
            let draw_outer_radius = outer_radius.clone();
            context.send_message(SceneControl::start_child_program(SubProgramId::new(), our_program_id, move |input, context|
                 pie_dialog_drawing_program(
                    input, 
                    context, 
                    (namespace, layer), 
                    draw_binding, 
                    draw_center, 
                    draw_angle,
                    draw_animation,
                    draw_inner_radius,
                    draw_outer_radius
                ), 3)).await.ok();

            // Tell SceneControl to create a subprogram to manage the focus regions of the slice
            let focus_focus_claims  = focus_claims.clone();
            let focus_center        = center.clone();
            let focus_angle         = angle.clone();
            let focus_inner_radius  = inner_radius.clone();
            let focus_outer_radius  = outer_radius.clone();
            let focus_z_index       = self.z_index;
            context.send_message(SceneControl::start_child_program(SubProgramId::new(), our_program_id, move |input, context|
                pie_dialog_focus_program(
                    input,
                    context,
                    point_mapping,
                    focus_focus_claims,
                    focus_center,
                    focus_angle,
                    focus_inner_radius,
                    focus_outer_radius,
                    focus_z_index,
                ), 3)).await.ok();

            // Process the input
            let point_mapping       = self.point_mapping();

            let mut input           = input;
            let mut send_drawing    = send_drawing;

            while let Some(msg) = input.next().await {
                match msg {
                    PieDialog::SetPosition(new_center, new_angle) => {
                        center.set(new_center);
                        angle.set(new_angle);
                    },

                    PieDialog::SetRadius(new_outer_radius) => {
                        outer_radius.set(new_outer_radius);
                    },

                    PieDialog::SetTitle(new_title) => {
                        title.set(new_title);
                    },

                    PieDialog::Draw(drawing) => {
                        for draw in drawing.iter().clone() {
                            send_drawing.send(draw.clone()).await.ok();
                        }
                    },

                    PieDialog::ClaimRegion { program, region, control, z_index } => {
                        // Transform the region
                        let region = region.into_iter()
                            .flat_map(|path| point_mapping.transform_path(path))
                            .collect::<Vec<_>>();

                        // Read the existing regions
                        let existing_regions        = focus_claims.get();
                        let mut existing_regions    = Arc::unwrap_or_clone(existing_regions);

                        // Add a new region
                        existing_regions.insert(control.clone(), PieFocusRegion { 
                            path:       region, 
                            control:    control, 
                            target:     program, 
                            z_index:    z_index 
                        });

                        // Update the binding (the subprogram will pick this up later on)
                        focus_claims.set(Arc::new(existing_regions));
                    },

                    PieDialog::Close => {
                        // TODO: animation
                        break;
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn map_zero_point() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        let zero_point = dialog_program.point_mapping().map_point(&UiPoint(0.0, 0.0));
        assert!((zero_point.x()-0.0).abs() < 0.1, "{:?}", zero_point);
        assert!((zero_point.y()-20.0).abs() < 0.1, "{:?}", zero_point);
    }

    #[test]
    pub fn map_furthest_point() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        let furthest_point = dialog_program.point_mapping().map_point(&UiPoint(0.0, 80.0));
        assert!((furthest_point.x()-0.0).abs() < 0.1, "{:?}", furthest_point);
        assert!((furthest_point.y()-100.0).abs() < 0.1, "{:?}", furthest_point);
    }

    #[test]
    pub fn unmap_points() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        fn check_unmap(program: &PieDialogProgram, point: UiPoint) {
            let mapped_point    = program.point_mapping().map_point(&point);
            let unmapped_point  = program.point_mapping().unmap_point(&mapped_point);

            assert!((point.x() - unmapped_point.x() < 0.01), "{:?} != {:?}", point, unmapped_point);
            assert!((point.y() - unmapped_point.y() < 0.01), "{:?} != {:?}", point, unmapped_point);
        }

        check_unmap(&dialog_program, UiPoint(0.0, 0.0));
        check_unmap(&dialog_program, UiPoint(0.0, 100.0));
        check_unmap(&dialog_program, UiPoint(50.0, 100.0));
        check_unmap(&dialog_program, UiPoint(-50.0, 0.0));
    }
}
