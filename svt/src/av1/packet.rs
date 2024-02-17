use svt_av1_sys::*;

use crate::Packet;

/// The type of a coded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Unknown or multiple.
    Unknown,
    /// Key frame.
    Key,
    /// Inter frame.
    Inter,
    /// Intra only frame.
    IntraOnly,
    /// An alternative reference frame.
    AltRef,
    // These are present in the source, but not used by the encoder.
    // ShowExisting,
    // ForwardKey,
    // Switch,
}

/// A packet of encoded data output by the encoder. The buffer is reference
/// counted, and will be reused by the encoder once dropped.
pub struct Av1Packet {
    ptr: *mut EbBufferHeaderType,
    is_headers: bool,
}

impl std::fmt::Debug for Av1Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("frame_type", &self.frame_type())
            .field("size", &unsafe { (*self.ptr).n_filled_len })
            .finish()
    }
}

impl Packet for Av1Packet {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts((*self.ptr).p_buffer, (*self.ptr).n_filled_len as usize)
        }
    }

    fn is_eos(&self) -> bool {
        unsafe { (*self.ptr).flags & EB_BUFFERFLAG_EOS != 0 }
    }
}

impl AsRef<[u8]> for Av1Packet {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Av1Packet {
    /// The type of frame in the output buffer.
    pub fn frame_type(&self) -> FrameType {
        unsafe {
            #[allow(non_upper_case_globals)]
            match (*self.ptr).pic_type {
                EbAv1PictureType_EB_AV1_KEY_PICTURE => FrameType::Key,
                EbAv1PictureType_EB_AV1_INTER_PICTURE | EbAv1PictureType_EB_AV1_NON_REF_PICTURE => {
                    FrameType::Inter
                }
                EbAv1PictureType_EB_AV1_INTRA_ONLY_PICTURE => FrameType::IntraOnly,
                EbAv1PictureType_EB_AV1_ALT_REF_PICTURE => FrameType::AltRef,
                _ => FrameType::Unknown,
            }
        }
    }

    pub(crate) fn new(p: *mut EbBufferHeaderType) -> Self {
        assert!(!p.is_null());

        Self {
            ptr: p,
            is_headers: false,
        }
    }

    pub(crate) fn new_headers(p: *mut EbBufferHeaderType) -> Self {
        assert!(!p.is_null());

        Self {
            ptr: p,
            is_headers: true,
        }
    }
}

impl Drop for Av1Packet {
    fn drop(&mut self) {
        unsafe {
            if self.is_headers {
                svt_av1_enc_stream_header_release(self.ptr);
            } else {
                svt_av1_enc_release_out_buffer(&mut self.ptr);
            }
        }
    }
}
