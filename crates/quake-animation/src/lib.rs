use glam::FloatExt;

#[derive(Clone, Debug)]
pub struct Animations {
    pub animations: Box<[Animation]>,
}

impl Animations {
    pub fn from_mdl_frames(frames: Box<[quake_model::mdl::Frame]>) -> Self {
        let mut animation = indexmap::IndexMap::new();
        for frame in frames.iter() {
            let animation_name = match frame {
                quake_model::mdl::Frame::Single(single) => {
                    single.name.trim_end_matches(char::is_numeric)
                }
                quake_model::mdl::Frame::Group(group) => group
                    .frames()
                    .first()
                    .unwrap()
                    .name
                    .trim_end_matches(char::is_numeric),
            };
            animation
                .entry(animation_name)
                .or_insert_with(Vec::new)
                .push(frame.clone());
        }

        let animations = animation
            .iter()
            .map(|(name, frames)| Animation::VertexAliasing {
                name: name.to_string(),
                frames: frames.to_vec().into_boxed_slice(),
            })
            .collect();

        Self { animations }
    }

    pub fn get_frame_by_index(&self, index: usize) -> Option<&quake_model::mdl::Frame> {
        let mut index = index;
        for animation in self.animations.iter() {
            match animation {
                Animation::VertexAliasing { frames, .. } => {
                    if index < frames.len() {
                        return Some(&frames[index]);
                    } else {
                        index -= frames.len();
                    }
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub enum Animation {
    VertexAliasing {
        name: String,
        frames: Box<[quake_model::mdl::Frame]>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Interpolation {
    None,
    Linear,
}

impl Interpolation {
    pub fn function(&self) -> fn(f32, f32, f32) -> f32 {
        match self {
            Interpolation::None => |a, _, _| a,
            Interpolation::Linear => f32::lerp,
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeyFrame {
    positions: Box<[glam::Vec3]>,
    bounding_volume: quake_collision::BoundingVolume,
}

impl KeyFrame {
    pub fn interpolate_with<F>(&self, other: &Self, t: f32, interpolate_fn: F) -> Self
    where
        F: Fn(f32, f32, f32) -> f32,
    {
        let positions_count = self.positions.len();
        let mut interpolated_positions = vec![glam::Vec3::ZERO; positions_count];
        for i in 0..positions_count {
            let p1 = self.positions[i];
            let p2 = other.positions[i];
            interpolated_positions[i] = Self::interpolate_vec3(&p1, &p2, t, &interpolate_fn);
        }

        let interpolated_bounding_volume =
            self.bounding_volume
                .interpolate_with(&other.bounding_volume, t, &interpolate_fn);

        Self {
            positions: interpolated_positions.into_boxed_slice(),
            bounding_volume: interpolated_bounding_volume,
        }
    }

    fn interpolate_vec3<F>(
        v1: &glam::Vec3,
        v2: &glam::Vec3,
        t: f32,
        interpolate_fn: F,
    ) -> glam::Vec3
    where
        F: Fn(f32, f32, f32) -> f32,
    {
        [
            interpolate_fn(v1[0], v2[0], t),
            interpolate_fn(v1[1], v2[1], t),
            interpolate_fn(v1[2], v2[2], t),
        ]
        .into()
    }
}
