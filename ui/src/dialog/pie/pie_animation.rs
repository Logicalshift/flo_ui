///
/// How a pie dialog should be animated
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PieAnimation {
    /// Just pop into/out of existence
    None,

    /// Expand from its hub
    Expand,

    /// Fan out from the central point
    FanOut,

    /// First expand, then fan out
    ExpandFanOut,
}
