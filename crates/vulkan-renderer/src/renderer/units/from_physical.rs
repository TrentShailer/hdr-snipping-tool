pub trait FromPhysical<P: Into<f64>> {
    fn from_physical(value: [P; 2], screen_size: [u32; 2]) -> Self;
}

pub trait AddPhysical<P: Into<f64>> {
    fn add_physical(self, value: [P; 2], screen_size: [u32; 2]) -> Self;
}
