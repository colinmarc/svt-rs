use svt_hevc_sys::*;

use crate::Packet;

enum DropType {
    Headers,
    Output,
    Eos,
}

/// The type of a NAL unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaluType {
    /// Unknown or multiple.
    Unknown,
    /// A B frame.
    B,
    /// A P frame.
    P,
    /// An I frame.
    I,
    /// An IDR frame.
    IDR,
}

/// A packet of encoded data output by the encoder. The buffer is reference
/// counted, and will be reused by the encoder once dropped.
pub struct HevcPacket {
    handle: *mut EB_BUFFERHEADERTYPE,
    ty: DropType,
}

impl std::fmt::Debug for HevcPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("slice_type", &self.nalu_type())
            .field("nalu_type", &unsafe { (*self.handle).naluNalType })
            .field("size", &unsafe { (*self.handle).nFilledLen })
            .finish()
    }
}

impl Packet for HevcPacket {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts((*self.handle).pBuffer, (*self.handle).nFilledLen as usize)
        }
    }

    fn is_eos(&self) -> bool {
        unsafe { (*self.handle).nFlags & EB_BUFFERFLAG_EOS != 0 }
    }
}

impl AsRef<[u8]> for HevcPacket {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl HevcPacket {
    /// The type of NAL unit contained.
    pub fn nalu_type(&self) -> NaluType {
        unsafe {
            match (*self.handle).sliceType {
                EB_B_PICTURE => NaluType::B,
                EB_P_PICTURE => NaluType::P,
                EB_I_PICTURE => NaluType::I,
                EB_IDR_PICTURE => NaluType::IDR,
                _ => NaluType::Unknown,
            }
        }
    }

    pub(crate) fn new(p: *mut EB_BUFFERHEADERTYPE) -> Self {
        Self {
            handle: p,
            ty: DropType::Output,
        }
    }

    pub(crate) fn new_headers(p: *mut EB_BUFFERHEADERTYPE) -> Self {
        Self {
            handle: p,
            ty: DropType::Headers,
        }
    }

    pub(crate) fn new_eos(p: *mut EB_BUFFERHEADERTYPE) -> Self {
        Self {
            handle: p,
            ty: DropType::Eos,
        }
    }
}

impl Drop for HevcPacket {
    fn drop(&mut self) {
        match self.ty {
            DropType::Headers => unsafe {
                EbH265EncReleaseStreamHeader(self.handle);
            },
            DropType::Output => unsafe {
                EbH265ReleaseOutBuffer(&mut self.handle);
            },
            DropType::Eos => unsafe {
                EbH265EncReleaseEosNal(self.handle);
            },
        }
    }
}
