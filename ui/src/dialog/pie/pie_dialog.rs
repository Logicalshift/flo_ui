use super::pie_animation::*;
use crate::subprograms::*;
use crate::util::*;

use flo_binding::*;
use flo_scene::*;
use flo_scene::programs::*;
use flo_draw::canvas::*;
use flo_curves::*;
use flo_curves::bezier::*;

use futures::prelude::*;
use futures::channel::mpsc;

use std::f64;
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
    pub fn with_title(mut self, new_title: String) -> Self {
        self.title = new_title;
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
    /// Maps a point from 'flat' space to 'pie' space
    ///
    #[inline]
    pub fn map_point<TCoord>(&self, pos: &TCoord) -> TCoord
    where 
        TCoord: Coordinate + Coordinate2D,
    {
        // The distance from the center is the y position plus the inner radius (so y=0 is the inner circle)
        let r = pos.y() + self.inner_radius;

        // The angle is 'x' distance around the pie from the 'angle'
        let theta = (pos.x()/(2.0*f64::consts::PI*r)) * 2.0*f64::consts::PI;
        let theta = self.angle + theta;

        // Calculate the new position from the old one
        let new_x = r * theta.sin() + self.center.x();
        let new_y = r * theta.cos() + self.center.y();

        TCoord::from_components(&[new_x, new_y])
    }

    ///
    /// Maps a point from 'pie' space to 'flat' space
    ///
    #[inline]
    pub fn unmap_point<TCoord>(&self, pos: &TCoord) -> TCoord
    where 
        TCoord: Coordinate + Coordinate2D,
    {
        let dx = pos.x() - self.center.x();
        let dy = pos.y() - self.center.y();

        let r     = (dx * dx + dy * dy).sqrt();
        let theta = dx.atan2(dy);

        let x = (theta - self.angle) * r;
        let y = r - self.inner_radius;

        TCoord::from_components(&[x, y])
    }

}

// Execution

impl PieDialogProgram {
    ///
    /// Runs the subprogram for this pie dialog
    ///
    pub async fn run(self, input: InputStream<PieDialog>, context: SceneContext) {
        // Namespace and layer that we'll be drawing on
        let namespace   = self.namespace;
        let layer       = self.layer;

        // Original angle and center (used when the position changes)
        let original_center = self.center;
        let original_angle  = self.angle;

        // Binding for the instructions used to redraw the pie slice (coordinates transformed)
        let draw_binding = bind(Arc::<Vec<Draw>>::new(vec![]));

        // Current position is used to calculate the layer transform
        let current_pos = bind((original_center, original_angle));

        // Title is rendered at the outer radius
        let outer_radius = bind(self.outer_radius);
        let title        = bind(self.title.clone());

        // Stream for processing the draw instructions (winds up owning 'self')
        let (send_drawing, recv_drawing) = mpsc::channel::<Draw>(1000);

        let with_dashed_lines   = drawing_without_dashed_lines(recv_drawing);
        let with_text_layout    = drawing_with_laid_out_text(with_dashed_lines);
        let with_glyph_paths    = drawing_with_text_as_paths(with_text_layout);
        let as_paths            = drawing_to_attributed_paths::<UiPath, _>(with_glyph_paths);
        let with_transform      = as_paths.map(move |(attributes, path_set)| {
                let new_paths = path_set.iter().flat_map(|path| distort_path::<_, _, UiPath>(path, |point, _, _| self.map_point(&point), 1.0, 0.1))
                    .collect::<Vec<_>>();
                (attributes, new_paths)
            });
        let redrawn_paths       = with_transform.flat_map(|(attributes, path_set)| {
            let mut draw = vec![];
            draw.render_bezier_shape(attributes.iter(), path_set.iter());
            stream::iter(draw)
        });

        // Process the input
        let mut input = input;

        while let Some(msg) = input.next().await {
            match msg {
                PieDialog::SetPosition(new_center, new_angle) => {
                    current_pos.set((new_center, new_angle));
                },

                PieDialog::SetRadius(new_outer_radius) => {
                    outer_radius.set(new_outer_radius);
                },

                PieDialog::SetTitle(new_title) => {
                    title.set(new_title);
                },

                PieDialog::Draw(drawing) => {
                    // TODO: transform paths according to map_point
                    // TODO: resources like fonts, etc get passed through
                    // TOOD: also need to convert text to paths for this conversion
                    todo!()
                },

                PieDialog::ClaimRegion { program, region, control, z_index } => {
                    // Need to transform/recalculate the path and then add to the claims
                    // (Also need a subprogram to manage them)
                    todo!()
                },

                PieDialog::Close => {
                    // TODO: animation
                    break;
                },
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

        let zero_point = dialog_program.map_point(&UiPoint(0.0, 0.0));
        assert!((zero_point.x()-100.0).abs() < 0.1, "{:?}", zero_point);
        assert!((zero_point.y()-120.0).abs() < 0.1, "{:?}", zero_point);
    }

    #[test]
    pub fn map_furthest_point() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        let furthest_point = dialog_program.map_point(&UiPoint(0.0, 80.0));
        assert!((furthest_point.x()-100.0).abs() < 0.1, "{:?}", furthest_point);
        assert!((furthest_point.y()-200.0).abs() < 0.1, "{:?}", furthest_point);
    }

    #[test]
    pub fn unmap_points() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        fn check_unmap(program: &PieDialogProgram, point: UiPoint) {
            let mapped_point    = program.map_point(&point);
            let unmapped_point  = program.unmap_point(&mapped_point);

            assert!((point.x() - unmapped_point.x() < 0.01), "{:?} != {:?}", point, unmapped_point);
            assert!((point.y() - unmapped_point.y() < 0.01), "{:?} != {:?}", point, unmapped_point);
        }

        check_unmap(&dialog_program, UiPoint(0.0, 0.0));
        check_unmap(&dialog_program, UiPoint(0.0, 100.0));
        check_unmap(&dialog_program, UiPoint(50.0, 100.0));
        check_unmap(&dialog_program, UiPoint(-50.0, 0.0));
    }
}
