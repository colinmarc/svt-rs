use crate::Plane;

/// A reusable YUV picture buffer, with each of the three planes as a separate
/// `Vec<u8>` and no support for row padding.
pub struct YUVBuffer {
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    uv_stride: u32,
    width: u32,
    height: u32,
}

impl std::fmt::Debug for YUVBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YUVBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl YUVBuffer {
    /// Create a new YUV picture with the given subs width and height.
    pub fn new(width: u32, height: u32, format: super::SubsamplingFormat) -> Self {
        let y_size = (width * height) as usize;

        let uv_width = match format {
            super::SubsamplingFormat::Yuv400 => 0,
            super::SubsamplingFormat::Yuv420 => width / 2,
            super::SubsamplingFormat::Yuv422 => width / 2,
            super::SubsamplingFormat::Yuv444 => width,
        };

        let uv_height = match format {
            super::SubsamplingFormat::Yuv400 => 0,
            super::SubsamplingFormat::Yuv420 => height / 2,
            super::SubsamplingFormat::Yuv422 => height,
            super::SubsamplingFormat::Yuv444 => height,
        };

        let uv_size = (uv_width * uv_height) as usize;

        YUVBuffer {
            y: vec![0; y_size],
            u: vec![0; uv_size],
            v: vec![0; uv_size],
            uv_stride: uv_width,
            width,
            height,
        }
    }

    /// Get mutable access to a plane.
    pub fn as_mut_slice(&mut self, plane: Plane) -> &mut [u8] {
        match plane {
            Plane::Y => &mut self.y,
            Plane::U => &mut self.u,
            Plane::V => &mut self.v,
        }
    }
}

impl super::Picture for YUVBuffer {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn as_slice(&self, plane: Plane) -> &[u8] {
        match plane {
            Plane::Y => &self.y,
            Plane::U => &self.u,
            Plane::V => &self.v,
        }
    }

    fn stride(&self, plane: Plane) -> u32 {
        match plane {
            Plane::Y => self.width,
            Plane::U | Plane::V => self.uv_stride,
        }
    }
}
