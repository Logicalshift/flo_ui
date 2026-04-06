use super::pie_animation::*;
use crate::subprograms::*;

use flo_scene::*;
use flo_draw::canvas::*;

use std::f64;

///
/// Configuration and a way to run pie dialogs
///
#[derive(Clone, Debug)]
pub struct PieDialogProgram {
    /// The animation used to show and hide this dialog
    animation:      PieAnimation,

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
    pub fn new(namespace: NamespaceId, layer: LayerId) -> Self {
        Self::default()
            .with_namespace(namespace)
            .with_layer(layer)
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
}

// Execution

impl PieDialogProgram {
    ///
    /// Runs the subprogram for this pie dialog
    ///
    pub async fn run(mut self, input: InputStream<PieDialog>, context: SceneContext) {

    }
}
