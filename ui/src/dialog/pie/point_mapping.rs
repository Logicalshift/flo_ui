use crate::util::*;

use flo_draw::canvas::*;
use flo_curves::*;
use flo_curves::bezier::*;
use flo_curves::bezier::path::*;

use futures::prelude::*;

use std::f64;

///
/// Describes how a point is mapped in a pie dialog
///
#[derive(Clone, Copy, Debug)]
pub struct PieDialogPointMapping {
    pub (super) inner_radius: f64,
    pub (super) outer_radius: f64,
}

impl PieDialogPointMapping {
    ///
    /// Maps a point from 'flat' space to 'pie' space
    ///
    #[inline]
    pub fn map_point<TCoord>(&self, pos: &TCoord) -> TCoord
    where 
        TCoord: Coordinate + Coordinate2D,
    {
        let width   = (self.outer_radius + self.inner_radius)*f64::consts::PI/4.0;
        let height  = self.outer_radius - self.inner_radius;

        // We provide a coordinate scheme that's -100 - +100 in the x range
        let theta = pos.x()*(0.5*f64::consts::PI)/width;
        let theta = theta;

        // The distance from the center scales as we get further away
        let r_min = self.inner_radius;
        let r_max = self.outer_radius;

        let r = if r_min > 0.0 {
            r_min * (r_max / r_min).powf(pos.y() / height)
        } else {
            pos.y() * (r_max / height)
        };

        // Calculate the new position from the old one
        let new_x = r * theta.sin();
        let new_y = r * theta.cos();

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
        let width   = (self.outer_radius + self.inner_radius)*f64::consts::PI/4.0;
        let height  = self.outer_radius - self.inner_radius;
        let r_min   = self.inner_radius;
        let r_max   = self.outer_radius;

        let dx = pos.x();
        let dy = pos.y();

        let r     = (dx * dx + dy * dy).sqrt();
        let theta = dx.atan2(dy);

        let x = theta * width / (0.5 * f64::consts::PI);
        let y = if r_min > 0.0 {
            height * (r / r_min).ln() / (r_max / r_min).ln()
        } else {
            r * height / r_max
        };

        TCoord::from_components(&[x, y])
    }

    ///
    /// Transforms any paths found in the supplied drawing, returning a new drawing (which just draws the paths)
    ///
    /// Things like layer clearing and resource operations will need to be processed separately from this.
    ///
    pub async fn transform_paths(&self, drawing: impl 'static + Send + Unpin + Stream<Item=Draw>) -> Vec<Draw> {
        let as_paths    = drawing_to_attributed_paths::<UiPath, _>(drawing);
        let transformed = as_paths.map(|(attributes, path_set)| {
                let new_paths = path_set.iter().flat_map(|path| distort_path::<_, _, UiPath>(path, |point, _, _| self.map_point(&point), 1.0, 0.1))
                    .collect::<Vec<_>>();
                (attributes, new_paths)
            });
        let redrawn     = transformed.flat_map(|(attributes, path_set)| {
            let mut draw = vec![];
            draw.render_bezier_shape(attributes.iter(), path_set.iter());
            stream::iter(draw)
        });

        redrawn.collect::<_>().await
    }

    ///
    /// Transforms a single path to the coordinate scheme of the pie slice
    ///
    pub fn transform_path<TPath>(&self, path: TPath) -> Option<TPath>
    where 
        TPath:          BezierPathFactory,
        TPath::Point:   Coordinate + Coordinate2D,
    {
        distort_path(&path, |point, _, _| self.map_point(&point), 1.0, 0.1)
    }
}
