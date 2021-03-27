pub struct State {
    pub boids: Vec<Boid>,
}

impl State {
    pub fn update(&mut self, delta_t: &f32){
        for boid in self.boids.iter_mut(){
            *boid = State::step_boid(boid, delta_t);
        }
    }

    #[allow(dead_code)]
    fn step_boid(boid: &Boid, delta_t: &f32) -> Boid{
        Boid{
            position: boid.position + *delta_t * boid.velocity,
            velocity: boid.velocity,
            rotation: boid.rotation * exp(*delta_t * boid.angular_velocity) ,
            angular_velocity: boid.angular_velocity,
        }
    }
}


#[derive(Copy, Clone)]
pub struct Boid {
    pub position: cgmath::Vector3<f32>,
    pub velocity: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub angular_velocity: cgmath::Quaternion<f32>,
}

fn exp(q: cgmath::Quaternion<f32>) -> cgmath::Quaternion<f32> {
    use cgmath::InnerSpace;
    use std::f32::consts::E;
    let scalar = q.s;
    let bivector = q.v;
    let mag = q.magnitude();
    //cgmath::InnerSpace::magnitude(q)
    let new_scalar = E.powf(scalar)*mag.cos();
    let new_bivector: cgmath::Vector3<f32>;
    if mag == 0.0 {
        new_bivector = cgmath::Vector3::new(0.0, 0.0, 0.0);
    } else {
        new_bivector = (bivector/mag)*mag.sin();
    }
    cgmath::Quaternion::new(new_scalar, new_bivector.x, new_bivector.y, new_bivector.z)
}
