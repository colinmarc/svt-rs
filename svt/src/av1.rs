//! A rust wrapper for the Alliance for Open Media's Scalable Video Technology
//! for AV1 (SVT-AV1) video encoder.
//!
//! # Example
//! ```
//! # use svt::{Encoder, Packet, YUVBuffer, SubsamplingFormat};
//! # use svt::av1::{Av1EncoderConfig, RateControlMode};
//! # fn copy_frame(_: &mut YUVBuffer)
//! #     -> Result<i64, Box<dyn std::error::Error>> { Ok(0) }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let width = 800;
//! # let height = 600;
//! # let colorspace = SubsamplingFormat::Yuv420;
//! let encoder = Av1EncoderConfig::default()
//!     .preset(8)
//!     .rate_control_mode(RateControlMode::ConstantRateFactor(30))
//!     .create_encoder(width, height, colorspace)?;
//!
//! let mut buffer = YUVBuffer::new(width, height, colorspace);
//!
//! loop {
//!     // Copy the YUV data into the buffer from a file, network stream, etc.
//!     // The source will also provide the PTS (presentation timestamp).
//!     let pts = copy_frame(&mut buffer)?;
//!
//!     // Submit the input picture.
//!     encoder.send_picture(&buffer, pts, false)?;
//!     while let Some(packet) = encoder.get_packet(false)? {
//!         // Write the packet to a file or send it over the network.
//!     }
//!
//! #   break
//! }
//!
//! // Once all frames have been submitted, flush the encoder.
//! encoder.finish()?;
//!
//! while let Some(packet) = encoder.get_packet(true)? {
//!     // Handle the final packets the same way, but check `is_eos` to see if
//!     // the stream is finished.
//!     if packet.is_eos() {
//!         break;
//!     }
//! }
//!
//! # Ok(())
//! # }

use svt_av1_sys::*;

use crate::{Encoder, Error, Picture, Plane, SubsamplingFormat};

mod config;
mod packet;

pub use config::*;
pub use packet::*;

struct LibraryHandle(*mut EbComponentType);

impl LibraryHandle {
    fn as_ptr(&self) -> *mut EbComponentType {
        self.0
    }
}

impl Drop for LibraryHandle {
    fn drop(&mut self) {
        unsafe {
            svt_av1_enc_deinit_handle(self.0);
        }
    }
}

unsafe impl Send for LibraryHandle {}

/// An encoder instance.
pub struct Av1Encoder {
    handle: LibraryHandle,
    subsampling_format: SubsamplingFormat,
}

impl std::fmt::Debug for Av1Encoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Encoder")
            .field(&self.handle.as_ptr())
            .finish()
    }
}

impl Encoder<Av1Packet> for Av1Encoder {
    fn send_picture(
        &self,
        picture: &impl Picture,
        pts: i64,
        force_keyframe: bool,
    ) -> Result<(), Error> {
        let y = picture.as_slice(Plane::Y);
        let u = picture.as_slice(Plane::U);
        let v = picture.as_slice(Plane::V);

        let y_stride = picture.stride(Plane::Y);
        let u_stride = picture.stride(Plane::U);
        let v_stride = picture.stride(Plane::V);

        assert_eq!(y.len(), (y_stride * picture.height()) as usize);
        match self.subsampling_format {
            SubsamplingFormat::Yuv400 => {
                assert_eq!(u.len(), 0);
                assert_eq!(v.len(), 0);
            }
            SubsamplingFormat::Yuv420 => {
                assert_eq!(u.len(), (u_stride * picture.height() / 2) as usize);
                assert_eq!(v.len(), (v_stride * picture.height() / 2) as usize);
            }
            SubsamplingFormat::Yuv422 | SubsamplingFormat::Yuv444 => {
                assert_eq!(u.len(), (u_stride * picture.height()) as usize);
                assert_eq!(v.len(), (v_stride * picture.height()) as usize);
            }
        }

        let mut input_pic = EbSvtIOFormat {
            luma: picture.as_slice(Plane::Y).as_ptr() as *mut _,
            cb: picture.as_slice(Plane::U).as_ptr() as *mut _,
            cr: picture.as_slice(Plane::V).as_ptr() as *mut _,
            y_stride,
            cr_stride: u_stride,
            cb_stride: v_stride,
            ..Default::default()
        };

        let pic_type = if force_keyframe {
            EbAv1PictureType_EB_AV1_KEY_PICTURE
        } else {
            EbAv1PictureType_EB_AV1_INVALID_PICTURE
        };

        let mut input = EbBufferHeaderType {
            size: std::mem::size_of::<EbBufferHeaderType>() as u32,
            p_buffer: &mut input_pic as *mut _ as *mut u8,
            n_filled_len: (y.len() + u.len() + v.len()) as u32,
            pts,
            pic_type,
            ..Default::default()
        };

        unsafe { result(svt_av1_enc_send_picture(self.handle.as_ptr(), &mut input)) }
    }

    fn get_packet(&self, wait: bool) -> Result<Option<Av1Packet>, Error> {
        let mut p = std::ptr::null_mut();
        unsafe {
            #[allow(non_upper_case_globals)]
            match svt_av1_enc_get_packet(self.handle.as_ptr(), &mut p, wait as u8) {
                EbErrorType_EB_NoErrorEmptyQueue => return Ok(None),
                code => result(code)?,
            }

            Ok(Some(Av1Packet::new(p)))
        }
    }

    fn finish(&self) -> Result<(), Error> {
        let mut input = EbBufferHeaderType {
            flags: EB_BUFFERFLAG_EOS,
            ..Default::default()
        };

        unsafe { result(svt_av1_enc_send_picture(self.handle.as_ptr(), &mut input)) }
    }
}

impl Av1Encoder {
    /// Constructs an encoder from an existing pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that both pointers are valid, and the encoder has
    /// been initialized with `svt_av1_enc_init_handle` and `svt_av1_enc_init`.
    pub unsafe fn from_raw(
        handle: *mut EbComponentType,
        cfg: *mut EbSvtAv1EncConfiguration,
    ) -> Self {
        #[allow(non_upper_case_globals)]
        let subsampling_format = match (*cfg).encoder_color_format {
            EbColorFormat_EB_YUV400 => SubsamplingFormat::Yuv400,
            EbColorFormat_EB_YUV420 => SubsamplingFormat::Yuv420,
            EbColorFormat_EB_YUV422 => SubsamplingFormat::Yuv422,
            EbColorFormat_EB_YUV444 => SubsamplingFormat::Yuv444,
            _ => panic!("unsupported color format"),
        };

        Av1Encoder {
            handle: LibraryHandle(handle),
            subsampling_format,
        }
    }

    /// Generates a Sequence Header OBU.
    ///
    /// This is not generally necessary, as the encoder will automatically
    /// generate headers as needed.
    pub fn code_headers(&self) -> Result<Av1Packet, Error> {
        let mut p = std::ptr::null_mut();
        unsafe {
            result(svt_av1_enc_stream_header(self.handle.as_ptr(), &mut p))?;

            Ok(Av1Packet::new_headers(p))
        }
    }
}

impl Drop for Av1Encoder {
    fn drop(&mut self) {
        unsafe {
            svt_av1_enc_deinit(self.handle.as_ptr());
        }
    }
}

#[allow(non_upper_case_globals)]
pub(crate) fn result(code: EbErrorType) -> Result<(), Error> {
    match code {
        0 => Ok(()),
        // These are used for decoding only.
        // EbErrorType_EB_DecUnsupportedBitstream => Err(Error::UnsupportedBitstream),
        // EbErrorType_EB_DecNoOutputPicture => Err(Error::NoOutputPicture),
        // EbErrorType_EB_DecDecodingError => Err(Error::DecodingError),
        // EbErrorType_EB_Corrupt_Frame => Err(Error::CorruptFrame),
        EbErrorType_EB_ErrorInsufficientResources => Err(Error::InsufficientResources),
        EbErrorType_EB_ErrorUndefined => Err(Error::Undefined),
        EbErrorType_EB_ErrorInvalidComponent => Err(Error::InvalidComponent),
        EbErrorType_EB_ErrorBadParameter => Err(Error::BadParameter),
        EbErrorType_EB_ErrorDestroyThreadFailed => Err(Error::DestroyThreadFailed),
        EbErrorType_EB_ErrorSemaphoreUnresponsive => Err(Error::SemaphoreUnresponsive),
        EbErrorType_EB_ErrorDestroySemaphoreFailed => Err(Error::DestroySemaphoreFailed),
        EbErrorType_EB_ErrorCreateMutexFailed => Err(Error::CreateMutexFailed),
        EbErrorType_EB_ErrorMutexUnresponsive => Err(Error::MutexUnresponsive),
        EbErrorType_EB_ErrorDestroyMutexFailed => Err(Error::DestroyMutexFailed),
        _ => Err(Error::Unknown(code)),
    }
}

#[cfg(test)]
mod tests {
    use crate::YUVBuffer;

    use super::*;

    #[test]
    fn encode_frame() {
        simple_logger::init_with_env().ok();

        let enc = Av1EncoderConfig::default()
            .preset(7)
            .create_encoder(800, 600, SubsamplingFormat::Yuv420)
            .expect("failed to create encoder");

        let buf = YUVBuffer::new(800, 600, SubsamplingFormat::Yuv420);

        enc.send_picture(&buf, 0, false)
            .expect("failed to send picture");

        let _packet = enc.get_packet(false).expect("failed to get packet");

        enc.finish().expect("failed to finish");

        let _packet = enc.get_packet(true).expect("failed to get final packet");
    }

    #[test]
    fn create_encoder_error() {
        simple_logger::init_with_env().ok();

        let _enc = Av1EncoderConfig::default()
            .preset(99)
            .create_encoder(800, 600, SubsamplingFormat::Yuv420)
            .map(|_| ())
            .expect_err("EB_BadParameter");
    }
}
