use scrgb::ScRGB;

use crate::ScrgbTonemapper;

/// Presets for the tonemapping whitepoint.
#[derive(Debug, Copy, Clone)]
pub enum Whitepoint {
    /// The display's SDR Reference White.
    SdrReferenceWhite,

    /// The display's maximum luminance.
    MaximumLuminance,

    /// The capture's brightest component.
    InputMaximum,

    /// Custom whitepoint.
    Custom(ScRGB),
}

impl Whitepoint {
    pub fn as_human_readable(&self) -> String {
        match self {
            Whitepoint::SdrReferenceWhite => "SDR White",
            Whitepoint::MaximumLuminance => "Display Luminance",
            Whitepoint::InputMaximum => "Brightest Pixel",
            Whitepoint::Custom(_) => "Custom",
        }
        .to_string()
    }
}

impl ScrgbTonemapper {
    /// Returns the whitepoint based on the current curve target.
    pub fn get_whitepoint(&self) -> ScRGB {
        match self.curve_target {
            Whitepoint::SdrReferenceWhite => self.display.sdr_referece_white,
            Whitepoint::MaximumLuminance => self.display.luminance,
            Whitepoint::InputMaximum => self.brightest_component,
            Whitepoint::Custom(whitepoint) => whitepoint,
        }
    }

    /// Sets the curve target to a new whitepoint.
    pub fn set_curve_target(&mut self, curve_target: Whitepoint) {
        self.curve_target = curve_target;
    }

    /// Adjustst the current whitepoint by an amount.
    pub fn adjust_whitepoint(&mut self, amount: ScRGB) {
        let current_whitepoint = self.get_whitepoint();
        let adjusted_whitepoint = current_whitepoint + amount;

        let new_whitepoint = if adjusted_whitepoint < ScRGB(0.0) {
            ScRGB(0.0)
        } else {
            adjusted_whitepoint
        };

        self.curve_target = Whitepoint::Custom(new_whitepoint);
    }
}
