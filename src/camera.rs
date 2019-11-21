use cgmath::Deg;


pub struct Camera {
    /// Field of fiew. Note that this is the horizontal half-angle, i.e. fov = 45 means a 90 degree horizontal FOV.
    pub fov: Deg<f32>
}


impl Camera {
    /// Creates a new Camera.
    pub fn new() -> Camera {
        Camera {
            fov: Deg(45.0) // 90 degrees
        }
    }
}