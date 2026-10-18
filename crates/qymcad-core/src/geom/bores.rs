//! THE SIZES A MEASURED HOLE IS BROUGHT TO: what a person typed when the part was drawn, read back from a mesh or a
//! section of one.

/// The diameters of holes a size is brought to: the clearance holes for metric screws, fine and medium, and the
/// drills for tapping them. A printed hole comes out a little smaller than drawn, and its mesh smaller still - a circle
/// drawn by chords - so a measured 3.36 is the 3.4 it was drawn as.
const BORES: [f64; 24] = [2.5, 3.0, 3.2, 3.3, 3.4, 4.2, 4.3, 4.5, 5.0, 5.3, 5.5, 6.4, 6.6, 6.8, 8.4, 8.5, 9.0, 10.2, 10.5, 11.0, 13.0, 14.0, 17.0, 18.0];

/// How far a measured diameter may stand from one of `BORES` to be taken for it, in mm.
const BORE_NEAR: f64 = 0.15;

/// A size measured on a mesh brought to a tenth of a millimetre: what a person typed when the part was drawn.
pub fn rounded(raw: f64) -> f64 {
    (raw * 10.0).round() / 10.0
}

/// A hole's diameter brought to the nearest of `BORES` within `BORE_NEAR`, otherwise to a tenth of a millimetre.
pub fn rounded_bore(raw: f64) -> f64 {
    BORES.iter().copied().filter(|b| (b - raw).abs() <= BORE_NEAR).min_by(|a, b| (a - raw).abs().total_cmp(&(b - raw).abs())).unwrap_or_else(|| rounded(raw))
}
