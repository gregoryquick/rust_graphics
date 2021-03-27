use crate::state;

pub const U32_SIZE: wgpu::BufferAddress = std::mem::size_of::<u32>() as wgpu::BufferAddress;

#[derive(Copy, Clone)]
pub struct Vertex {
    #[allow(dead_code)]
    pub position: cgmath::Vector2<f32>,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float2
        ],
    };
}

//Creates buffers for boid drawing info
pub struct BoidBufferBuilder{
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
    pub current_boid: u32,
}

impl BoidBufferBuilder{
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            current_boid: 0,
        }
    }

    pub fn push_boid(mut self, boid: &state::Boid) -> Self {
    //pub fn push_boid(mut self) -> Self {
        //Add vertices for boid
        let pose_matrix = cgmath::Matrix4::from_translation(boid.position) * cgmath::Matrix4::from(boid.rotation);
        
        let vertex_positions: &[cgmath::Vector2<f32>; 3] = &[
            [0.000, 0.086,].into(),
            [-0.100, -0.086,].into(),
            [0.100, -0.086,].into(),
        ];

        let new_vertices: &[Vertex] = &vertex_positions.map(|x| Vertex {position: x,});
        
        self.vertex_data.extend(new_vertices);
        
        //Added index info for boid
        let new_indices = &[
            self.current_boid * 3 + 0,
            self.current_boid * 3 + 1,
            self.current_boid * 3 + 2,
        ];
        
        self.index_data.extend(new_indices);
        self.current_boid += 1;

        self
    }

    pub fn build(self, device: &wgpu::Device) -> (StagingBuffer, StagingBuffer, u32) {
        (
            StagingBuffer::new(device, &self.vertex_data),
            StagingBuffer::new(device, &self.index_data),
            self.index_data.len() as u32,
        )
    }


    //fn boid_to_pose(self) {
    //    ()
    //}
}


//Used for copying data to gpu
pub struct StagingBuffer {
    buffer: wgpu::Buffer,
    size: wgpu::BufferAddress,
}

use wgpu::util::{BufferInitDescriptor, DeviceExt};
impl StagingBuffer {
    pub fn new<T: bytemuck::Pod + Sized>(device: &wgpu::Device, data: &[T]) -> StagingBuffer {
        use wgpu::util::{BufferInitDescriptor, DeviceExt};
        StagingBuffer {
            buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Staging Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsage::COPY_SRC,
            }),
            size: size_of_slice(data) as wgpu::BufferAddress,
        }
    }

    pub fn copy_to_buffer(&self, encoder: &mut wgpu::CommandEncoder, other: &wgpu::Buffer) {
        encoder.copy_buffer_to_buffer(&self.buffer, 0, other, 0, self.size)
    }
}

pub fn size_of_slice<T: Sized>(slice: &[T]) -> usize {
    std::mem::size_of::<T>() * slice.len()
}



