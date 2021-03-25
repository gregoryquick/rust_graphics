pub struct State {
    pub boids: Vec<Boid>,
}

pub struct Boid {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

