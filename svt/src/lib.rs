//! Rust wrappers for Intel's SVT (Scalable Video Technology) family of encoders.

#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

mod buffer;
pub use buffer::YUVBuffer;

mod error;
pub use error::Error;

#[cfg(feature = "av1")]
pub mod av1;

#[cfg(feature = "hevc")]
pub mod hevc;

/// The chroma subsampling format of a YUV picture.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SubsamplingFormat {
    /// 4:0:0 Monochrome (with no chroma planes).
    Yuv400,
    /// 4:2:0 chroma subsampling.
    ///
    /// Each chroma plane is half the width and half the height of the luma plane.
    ///
    /// This is the most common format for video.
    Yuv420,
    /// 4:2:2 chroma subsampling.
    ///
    /// Each chroma plane is half the width of the luma plane, but the same height.
    Yuv422,
    /// 4:4:4 chroma subsampling.
    ///
    /// The chroma planes are the same size as the luma plane.
    Yuv444,
}

/// A plane of a YUV picture.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(usize)]
pub enum Plane {
    /// The luma plane.
    Y = 0,
    /// The first chroma plane.
    U = 1,
    /// The second chroma plane.
    V = 2,
}

/// A YUV picture, used as an encoder input frame.
///
/// Implementing this trait allows callers to provide their own data structures
/// to the encoder. For a ready-made implementation, see [`YUVBuffer`].
pub trait Picture {
    /// The width of the picture in pixels.
    fn width(&self) -> u32;

    /// The height of the picture in pixels.
    fn height(&self) -> u32;

    /// The raw bytes of a plane. The size depends on the chroma subsampling
    /// format. For the Y plane, it is always `stride` * `height` bytes.
    fn as_slice(&self, plane: Plane) -> &[u8];

    /// The stride, or row width, of a plane. Stride affects the number of bytes
    /// used to store a plane, but not the size of the picture in pixels.
    fn stride(&self, plane: Plane) -> u32;
}

/// A packet of encoded data output by the encoder.
pub trait Packet: AsRef<[u8]> + std::fmt::Debug {
    /// Access the encoded bytes.
    fn as_bytes(&self) -> &[u8];

    /// Whether this packet is the last one in the stream.
    fn is_eos(&self) -> bool;
}

/// An encoder generates compressed video bitstreams.
///
/// # Example
///
/// ```
/// # use svt::{YUVBuffer, Plane, Packet};
/// # #[derive(Debug)]
/// # struct DummyPacket;
/// # impl Packet for DummyPacket {
/// #     fn as_bytes(&self) -> &[u8] { &[] }
/// #     fn is_eos(&self) -> bool { true }
/// # }
/// # impl AsRef<[u8]> for DummyPacket {
/// #     fn as_ref(&self) -> &[u8] { &[] }
/// # }
/// # fn doctest(encoder: impl svt::Encoder<DummyPacket>) -> Result<(), svt::Error> {
/// loop {
///     // Get a picture from somewhere. The width, height, and subsampling
///     // format must match the encoder's configuration.
///     let mut picture = YUVBuffer::new(800, 600, svt::SubsamplingFormat::Yuv420);
///
///     // Fill the picture data.
///     let y = picture.as_mut_slice(Plane::Y);
///     // ... fill the Y plane with luma, and the other two planes with chroma.
///
///     // The presentation timestamp tells the decoder on the other end when
///     // to present the frame.
///     let pts = 0;
///
///     // Submit the input picture
///     encoder.send_picture(&picture, pts, false)?;
///     while let Some(packet) = encoder.get_packet(false)? {
///        // Write the packet to a file or send it over the network.
///     }
///
/// #   break
/// }
///
/// encoder.finish()?;
/// while let Some(packet) = encoder.get_packet(true)? {
///    // Handle the final packets the same way, but check for EOS.
///    if packet.is_eos() {
///        break;
///    }
/// }
/// # Ok(())
/// # }
/// ```
pub trait Encoder<P: Packet> {
    /// Sends an input picture to the encoder. The picture should have the same
    /// dimensions as the encoder, and the same chroma subsampling layout that
    /// the encoder was configured with (usually 4:2:0).
    ///
    /// `pts` is will be used as the presentation timestamp. `force_keyframe`
    /// will force the encoder to perform an intra refresh.
    fn send_picture(
        &self,
        picture: &impl Picture,
        pts: i64,
        force_keyframe: bool,
    ) -> Result<(), Error>;

    /// Requests that the encoder finish encoding and generate an EOS packet to
    /// end the stream.
    fn finish(&self) -> Result<(), Error>;

    /// Retrieves an encoded packet from the encoder.
    ///
    /// If `wait` is true, this function will block until a packet is available,
    /// or indefinitely if the stream is already finished. Therefore, Callers
    /// should check [`Packet::is_eos`] to determine when the stream has ended.
    fn get_packet(&self, wait: bool) -> Result<Option<P>, Error>;
}
