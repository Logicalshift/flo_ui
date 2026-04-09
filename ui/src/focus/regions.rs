use crate::subprograms::*;
use crate::util::*;

use flo_scene::*;

use flo_curves::geo::*;
use flo_curves::bezier::rasterize::*;
use flo_curves::bezier::vectorize::*;

///
/// Represents a control within a subprogram
///
pub (super) struct SubProgramControl {
    pub (super) id:             ControlId,
    pub (super) event_target:   SubProgramId,
    pub (super) bounds:         Bounds<UiPoint>,
    pub (super) region:         PathContour,
    pub (super) z_index:        usize,
}

///
/// Definition for a region of the canvas where a subprogram owns the events
///
pub (super) struct SubProgramRegion {
    pub (super) event_target:   SubProgramId,
    pub (super) region:         PathContour,
    pub (super) bounds:         Bounds<UiPoint>,
    pub (super) controls:       Vec<SubProgramControl>,
    pub (super) z_index:        usize,
}

///
/// Represents the ordering of subprograms that can receive keyboard focus
///
pub (super) struct KeyboardSubProgram {
    pub (super) control_order:  Vec<ControlId>,
}

impl SubProgramRegion {
    ///
    /// Returns true if the specified point is inside this region
    ///
    pub fn point_is_inside(&self, x: f64, y: f64) -> bool {
        // Note that PathContour doesn't support negative values for x

        let intercepts = self.region.intercepts_on_line(y);
        if intercepts.into_iter().any(|intercept| intercept.contains(&x)) {
            // Intercept in the region for this program
            true
        } else {
            // Could be an intercept on any of the controls in this region
            self.controls.iter()
                .filter(|control| UiPoint(x, y).in_bounds(&control.bounds))
                .any(|control| control.point_is_inside(x, y))
        }
    }
}

impl SubProgramControl {
    ///
    /// Returns true if the specified point is inside this region
    ///
    pub fn point_is_inside(&self, x: f64, y: f64) -> bool {
        // Note that PathContour doesn't support negative values for x

        let intercepts = self.region.intercepts_on_line(y);
        intercepts.into_iter().any(|intercept| intercept.contains(&x))
    }
}
