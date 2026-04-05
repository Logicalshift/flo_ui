use super::control_id::*;
use crate::util::*;

use flo_draw::canvas::*;
use flo_scene::*;

use serde::*;

use std::sync::*;

///
/// A 'pie dialog' is a dialog box that is mapped into a circle.
///
/// These are the raw messages that can be exchanged with the main pie dialog program.
///
#[derive(Serialize, Deserialize)]
pub enum PieDialog {
    /// Sets the central point of the pie dialog and the central angle
    SetPosition(UiPoint, f64),

    /// Sets the outer radius of the pie dialog
    SetRadius(f64),

    /// Sets the title of the pie dialog (displayed on the outer rim)
    SetTitle(String),

    /// Renders to the pie dialog, converting to the polar coordinates for the 
    Draw(Arc<Vec<Draw>>),

    /// Claims a control region within the pie (the region is converted to the polar coordinate scheme)
    ClaimRegion { program: SubProgramId, region: Vec<UiPath>, control: ControlId, z_index: usize },

    /// Animates the dialog closed, then stops the pie dialog program
    Close,
}
