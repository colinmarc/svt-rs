//! A rust wrapper for Intel's Scalable Video Technology for HEVC (SVT-HEVC)
//! video encoder.
//!
//! # Example
//! ```
//! # use svt::{Encoder, Packet, YUVBuffer, SubsamplingFormat};
//! # use svt::hevc::{HevcEncoderConfig, RateControlMode};
//! # fn copy_frame(_: &mut YUVBuffer)
//! #     -> Result<i64, Box<dyn std::error::Error>> { Ok(0) }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let width = 800;
//! # let height = 600;
//! # let colorspace = SubsamplingFormat::Yuv420;
//! let encoder = HevcEncoderConfig::default()
//!     .preset(8)
//!     .rate_control_mode(RateControlMode::ConstantQp)
//!     .qp(30)
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

use svt_hevc_sys::*;

mod config;
mod packet;

pub use config::*;
pub use packet::*;

use crate::{Encoder, Error, Picture, Plane, SubsamplingFormat};

struct LibraryHandle(*mut EB_COMPONENTTYPE);

unsafe impl Send for LibraryHandle {}

impl LibraryHandle {
    fn as_ptr(&self) -> *mut EB_COMPONENTTYPE {
        self.0
    }
}

impl Drop for LibraryHandle {
    fn drop(&mut self) {
        unsafe {
            EbDeinitHandle(self.0);
        }
    }
}

/// An encoder instance.
pub struct HevcEncoder {
    handle: LibraryHandle,
    subsampling_format: SubsamplingFormat,
    intra_refresh_type: IntraRefreshType,
}

impl std::fmt::Debug for HevcEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Encoder")
            .field(&self.handle.as_ptr())
            .finish()
    }
}

impl Encoder for HevcEncoder {
    type Packet = HevcPacket;

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

        let mut input_pic = EB_H265_ENC_INPUT {
            luma: picture.as_slice(Plane::Y).as_ptr() as *mut _,
            cb: picture.as_slice(Plane::U).as_ptr() as *mut _,
            cr: picture.as_slice(Plane::V).as_ptr() as *mut _,
            yStride: y_stride,
            crStride: u_stride,
            cbStride: v_stride,
            ..Default::default()
        };

        let slice_type = if force_keyframe {
            match self.intra_refresh_type {
                IntraRefreshType::Open => EB_I_PICTURE,
                IntraRefreshType::Closed(_) => EB_IDR_PICTURE,
            }
        } else {
            EB_INVALID_PICTURE
        };

        let mut input = EB_BUFFERHEADERTYPE {
            nSize: size_of::<EB_BUFFERHEADERTYPE>() as u32,
            pBuffer: &mut input_pic as *mut _ as *mut u8,
            nFilledLen: (y.len() + u.len() + v.len()) as u32,
            pts,
            sliceType: slice_type,
            ..Default::default()
        };

        unsafe { result(EbH265EncSendPicture(self.handle.as_ptr(), &mut input)) }
    }

    fn get_packet(&self, done: bool) -> Result<Option<HevcPacket>, Error> {
        let mut p = std::ptr::null_mut();
        unsafe {
            #[allow(non_upper_case_globals)]
            match EbH265GetPacket(self.handle.as_ptr(), &mut p, done as u8) {
                EB_ERRORTYPE_EB_NoErrorEmptyQueue => return Ok(None),
                code => result(code)?,
            }

            Ok(Some(HevcPacket::new(p)))
        }
    }

    fn finish(&self) -> Result<(), Error> {
        let mut input = EB_BUFFERHEADERTYPE {
            nFlags: EB_BUFFERFLAG_EOS,
            ..Default::default()
        };

        unsafe { result(EbH265EncSendPicture(self.handle.as_ptr(), &mut input)) }
    }
}

impl HevcEncoder {
    /// Constructs an encoder from an existing pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that both pointers are valid, and the encoder has
    /// been initialized with `EbInitHandle` and `EbInitEncoder`.
    pub unsafe fn from_raw(
        handle: *mut EB_COMPONENTTYPE,
        cfg: *mut EB_H265_ENC_CONFIGURATION,
    ) -> Self {
        let subsampling_format = match (*cfg).encoderColorFormat {
            0 => SubsamplingFormat::Yuv400,
            1 => SubsamplingFormat::Yuv420,
            2 => SubsamplingFormat::Yuv422,
            3 => SubsamplingFormat::Yuv444,
            _ => panic!("invalid subsampling format"),
        };

        let intra_refresh_type = match (*cfg).intraRefreshType {
            -1 => IntraRefreshType::Open,
            v => IntraRefreshType::Closed(v),
        };

        Self {
            handle: LibraryHandle(handle),
            subsampling_format,
            intra_refresh_type,
        }
    }

    /// Generates a VPS/SPS/PPS header NAL unit.
    ///
    /// This is not generally necessary, as the encoder will automatically
    /// generate headers as needed.
    pub fn code_headers(&self) -> Result<HevcPacket, Error> {
        let mut p = std::ptr::null_mut();
        unsafe {
            result(EbH265EncStreamHeader(self.handle.as_ptr(), &mut p))?;

            Ok(HevcPacket::new_headers(p))
        }
    }

    /// Generates an EOS (end-of-stream) NAL unit.
    ///
    /// This is not generally necessary, as the encoder will automatically
    /// generate EOS NAL units at the end of the stream.
    pub fn code_eos(&self) -> Result<HevcPacket, Error> {
        let mut p = std::ptr::null_mut();
        unsafe {
            result(EbH265EncEosNal(self.handle.as_ptr(), &mut p))?;

            Ok(HevcPacket::new_eos(p))
        }
    }
}

impl Drop for HevcEncoder {
    fn drop(&mut self) {
        unsafe {
            EbDeinitEncoder(self.handle.as_ptr());
        }
    }
}

#[allow(non_upper_case_globals)]
pub(crate) fn result(code: EB_ERRORTYPE) -> Result<(), Error> {
    match code {
        0 => Ok(()),
        EB_ERRORTYPE_EB_ErrorInsufficientResources => Err(Error::InsufficientResources),
        EB_ERRORTYPE_EB_ErrorUndefined => Err(Error::Undefined),
        EB_ERRORTYPE_EB_ErrorInvalidComponent => Err(Error::InvalidComponent),
        EB_ERRORTYPE_EB_ErrorBadParameter => Err(Error::BadParameter),
        EB_ERRORTYPE_EB_ErrorDestroyThreadFailed => Err(Error::DestroyThreadFailed),
        EB_ERRORTYPE_EB_ErrorSemaphoreUnresponsive => Err(Error::SemaphoreUnresponsive),
        EB_ERRORTYPE_EB_ErrorDestroySemaphoreFailed => Err(Error::DestroySemaphoreFailed),
        EB_ERRORTYPE_EB_ErrorCreateMutexFailed => Err(Error::CreateMutexFailed),
        EB_ERRORTYPE_EB_ErrorMutexUnresponsive => Err(Error::MutexUnresponsive),
        EB_ERRORTYPE_EB_ErrorDestroyMutexFailed => Err(Error::DestroyMutexFailed),
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

        let enc = HevcEncoderConfig::default()
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

        let _enc = HevcEncoderConfig::default()
            .preset(99)
            .create_encoder(800, 600, SubsamplingFormat::Yuv420)
            .map(|_| ())
            .expect_err("EB_BadParameter");
    }
}
