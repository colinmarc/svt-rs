use svt_av1_sys::*;

use crate::{Error, SubsamplingFormat};

use super::{result, Av1Encoder, LibraryHandle};

mod cpu_flags;
pub use cpu_flags::CpuFlags;

/// How often (in frames) to insert an intra refresh.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IntraPeriod {
    /// Automatically determine the intra period.
    Auto,
    /// Do not insert intra refresh.
    None,
    /// Use a fixed intra period.
    Fixed(u32),
}

/// The type of intra refresh to use.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IntraRefreshType {
    /// Open, using CRA points for random access.
    Open,
    /// Closed, using IDR frames and a fixed GOP size.
    Closed,
}

/// The prediction structure for each GOP.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PredictionStructure {
    /// Forward prediction only.
    LowDelay,
    /// Bidirectional prediction.
    RandomAccess,
}

/// The AV1 bitstream profile.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Av1Profile {
    /// Main profile.
    Main,
    /// High profile.
    High,
    /// Professional profile.
    Professional,
}

/// AV1 decoder tier.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Av1Tier {
    /// Main, for most applications.
    Main,
    /// High, for demanding applications.
    High,
}

/// Input/output color space, according to SO/IEC 23091-4/ITU-T H.273.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Colorspace {
    /// Unspecified color space (CP_UNSPECIFIED, TC_UNSPECIFIED, MC_UNSPECIFIED).
    Unspecified,
    /// CP_BT_709 color primaries, TC_BT_709 transfer characteristics, and MC_BT_709 matrix coefficients. Standard for HD.
    Bt709,
    /// CP_BT_2020 color primaries, TC_BT_2020_10_BIT transfer characteristics, and MC_BT_2020_NCL matrix coefficients. Standard for UHD.
    Bt2020,
    /// Some other combination. See the AV1 spec section 6.4.2 for details.
    Other {
        /// The color primaries.
        primaries: u32,
        /// The transfer characteristics.
        transfer_characteristics: u32,
        /// The matrix coefficients.
        matrix_coefficients: u32,
    },
}

/// Input/output color range.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ColorRange {
    /// Studio swing (16-235 for Y, 16-240 for U and V).
    Limited,
    /// Full swing (0-255 for Y, 0-255 for U and V).
    Full,
}

/// Chroma sample position.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChromaSamplePosition {
    /// Top left.
    Colocated,
    /// Left.
    Vertical,
    /// Unknown.
    Unknown,
}

/// The rate control mode to use.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RateControlMode {
    /// Use a constant QP (1-63).
    ConstantQp(u32),
    /// Use a constant rate factor to hit a target QP (1-63).
    ConstantRateFactor(u32),
    /// Use variable bitrate. The value is in bits per second.
    VariableBitrate(u32),
    /// Use a constant bitrate. The value is in bits per second.
    ConstantBitrate(u32),
}

/// The strength of the constrained directional enhancement filter.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CdefLevel {
    /// Disable the filter.
    Off,
    /// Auto-select the filter strength.
    Auto,
    /// Enable the filter with a strength of 1-4.
    Enable(u32),
}

/// The restoration filtering mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RestorationFilteringMode {
    /// Disable restoration filtering.
    Off,
    /// Enable restoration filtering.
    On,
    /// Auto-select the restoration filtering mode.
    Auto,
}

/// The tiling mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TilingMode {
    /// Use a single tile.
    Single,
    /// Use multiple tiles.
    Multi {
        /// The number of columns.
        columns: u32,
        /// The number of rows.
        rows: u32,
    },
}

/// The recode loop level.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RecodeLevel {
    /// Disable recode loops.
    Disable,
    /// Enable recoding for keyframes and when exceeding maximum bandwidth.
    EnableForKeyframesAndBandwidth,
    /// Enable recoding for keyframes, alt-ref frames, and golden frames.
    EnableForKeyframes,
    /// Enable for all frames.
    EnableAll,
    /// Auto-select the recode loop level based on the preset.
    Auto,
}

/// The tuning metric.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tune {
    /// Visual quality.
    Vq,
    /// PSNR (peak signal-to-noise ratio).
    Psnr,
    /// SSIM (structural similarity index).
    Ssim,
}

/// The method and interval for switch frame insertion.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SwitchFrameInsertion {
    /// Disable switch frame insertion.
    Disabled,
    /// Insert switch frames at the given interval, converting the frame only if it is an alt-ref frame.
    Strict(u32),
    /// Insert switch frames at the given interval, converting the next alt-ref frame.
    Nearest(u32),
}

/// Which socket(s) to use for encoding, on dual-socket systems.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TargetSocket {
    /// Use the first socket.
    First,
    /// Use the second socket.
    Second,
    /// Use both sockets.
    Both,
}

/// A helper for building an encode configuration.
///
/// For configuration options, see the upstream docs:
///
/// <https://gitlab.com/AOMediaCodec/SVT-AV1/-/blob/master/Docs/Parameters.md?ref_type=heads>
pub struct Av1EncoderConfig {
    handle: LibraryHandle,
    cfg: EbSvtAv1EncConfiguration,
}

impl Default for Av1EncoderConfig {
    fn default() -> Self {
        unsafe {
            let mut handle = std::ptr::null_mut();
            let mut cfg = std::mem::zeroed();

            let res = svt_av1_enc_init_handle(&mut handle, std::ptr::null_mut(), &mut cfg);
            assert_eq!(0, res);

            Av1EncoderConfig {
                handle: LibraryHandle(handle),
                cfg,
            }
        }
    }
}

impl std::fmt::Debug for Av1EncoderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EncoderConfig")
            .field(&self.handle.as_ptr())
            .finish()
    }
}

impl Av1EncoderConfig {
    /// Creates a new encoder from the config.
    pub fn create_encoder(
        mut self,
        width: u32,
        height: u32,
        subsampling_format: SubsamplingFormat,
    ) -> Result<Av1Encoder, Error> {
        // Set the frame size.
        self.cfg.source_width = width;
        self.cfg.source_height = height;
        self.cfg.encoder_color_format = match subsampling_format {
            SubsamplingFormat::Yuv400 => 0,
            SubsamplingFormat::Yuv420 => 1,
            SubsamplingFormat::Yuv422 => 2,
            SubsamplingFormat::Yuv444 => 3,
        };

        // Copy config parameters onto the encoder handle.
        unsafe {
            result(svt_av1_enc_set_parameter(
                self.handle.as_ptr(),
                &mut self.cfg,
            ))?
        }

        // Create the encoder.
        unsafe { result(svt_av1_enc_init(self.handle.as_ptr()))? }

        Ok(Av1Encoder {
            handle: self.handle,
            subsampling_format: match self.cfg.encoder_color_format {
                0 => SubsamplingFormat::Yuv400,
                1 => SubsamplingFormat::Yuv420,
                2 => SubsamplingFormat::Yuv422,
                3 => SubsamplingFormat::Yuv444,
                _ => unreachable!(),
            },
        })
    }

    /// Sets the encoder preset, from 0-13, with 0 being the highest quality and
    /// 13 the fastest.
    pub fn preset(mut self, preset: i8) -> Self {
        self.cfg.enc_mode = preset;
        self
    }

    /// Sets the intra refresh period.
    pub fn intra_period_length(mut self, intra_period_length: IntraPeriod) -> Self {
        self.cfg.intra_period_length = match intra_period_length {
            IntraPeriod::Auto => -2,
            IntraPeriod::None => -1,
            IntraPeriod::Fixed(frames) => frames as i32,
        };

        self
    }

    /// Sets the intra refresh type.
    pub fn intra_refresh_type(mut self, intra_refresh_type: IntraRefreshType) -> Self {
        self.cfg.intra_refresh_type = match intra_refresh_type {
            IntraRefreshType::Open => 1,
            IntraRefreshType::Closed => 2,
        };

        self
    }

    /// Sets the number of hierarchical layers to use in each GOP.
    pub fn hierarchical_levels(mut self, hierarchical_levels: u32) -> Self {
        self.cfg.hierarchical_levels = hierarchical_levels;
        self
    }

    /// Configures the prediction structure for each GOP.
    pub fn pred_structure(mut self, pred_structure: PredictionStructure) -> Self {
        self.cfg.pred_structure = match pred_structure {
            PredictionStructure::LowDelay => 1,
            PredictionStructure::RandomAccess => 2,
        };

        self
    }

    /// Set the maximum width and height to communicate in headers. This is
    /// generally only needed for producing multiple renditions with switch
    /// frames.
    pub fn set_forced_max_frame_size(mut self, width: u32, height: u32) -> Self {
        self.cfg.forced_max_frame_width = width;
        self.cfg.forced_max_frame_height = height;
        self
    }

    /// Sets the input framerate.
    pub fn framerate(mut self, numerator: u32, denominator: u32) -> Self {
        self.cfg.frame_rate_numerator = numerator;
        self.cfg.frame_rate_denominator = denominator;
        self
    }

    /// Sets the input bit depth (8 or 10).
    pub fn bit_depth(mut self, bit_depth: u32) -> Self {
        self.cfg.encoder_bit_depth = bit_depth;
        self
    }

    /// Sets the AV1 profile.
    pub fn profile(mut self, profile: Av1Profile) -> Self {
        self.cfg.profile = match profile {
            Av1Profile::Main => 0,
            Av1Profile::High => 1,
            Av1Profile::Professional => 2,
        };

        self
    }

    /// Enables the usage of high-tier AV1 features.
    pub fn tier(mut self, tier: Av1Tier) -> Self {
        self.cfg.tier = match tier {
            Av1Tier::Main => 0,
            Av1Tier::High => 1,
        };

        self
    }

    /// Sets the AV1 level. A value of 0 configures the encoder to auto-detect
    /// the level. Any other value represents the level multiplied by ten - for
    /// example, 31 for level 3.1.
    pub fn level(mut self, level: u32) -> Self {
        self.cfg.level = level;
        self
    }

    /// Sets the color space to tag the bitstream with.
    pub fn color_space(mut self, color_space: Colorspace) -> Self {
        let (cp, tc, mc) = match color_space {
            Colorspace::Unspecified => (2, 2, 2),
            Colorspace::Bt709 => (1, 1, 1),
            Colorspace::Bt2020 => (9, 14, 9),
            Colorspace::Other {
                primaries,
                transfer_characteristics,
                matrix_coefficients,
            } => (primaries, transfer_characteristics, matrix_coefficients),
        };

        self.cfg.color_primaries = cp;
        self.cfg.transfer_characteristics = tc;
        self.cfg.matrix_coefficients = mc;

        self
    }

    /// Sets the color range to tag the bitstream with.
    pub fn color_range(mut self, color_range: ColorRange) -> Self {
        self.cfg.color_range = match color_range {
            ColorRange::Limited => 1,
            ColorRange::Full => 0,
        };

        self
    }

    /// Sets the chroma sample position to tag the bitstream with.
    pub fn chroma_sample_position(mut self, chroma_sample_position: ChromaSamplePosition) -> Self {
        self.cfg.chroma_sample_position = match chroma_sample_position {
            ChromaSamplePosition::Colocated => EbChromaSamplePosition_EB_CSP_COLOCATED,
            ChromaSamplePosition::Vertical => EbChromaSamplePosition_EB_CSP_VERTICAL,
            ChromaSamplePosition::Unknown => EbChromaSamplePosition_EB_CSP_UNKNOWN,
        };

        self
    }

    /// Sets the rate control mode.
    pub fn rate_control_mode(mut self, rate_control_mode: RateControlMode) -> Self {
        match rate_control_mode {
            RateControlMode::ConstantQp(qp) => {
                self.cfg.rate_control_mode = 0;
                self.cfg.enable_adaptive_quantization = 0;
                self.cfg.qp = qp;
            }
            RateControlMode::ConstantRateFactor(qp) => {
                self.cfg.rate_control_mode = 0;
                self.cfg.enable_adaptive_quantization = 1;
                self.cfg.qp = qp;
            }
            RateControlMode::VariableBitrate(bitrate) => {
                self.cfg.rate_control_mode = 1;
                self.cfg.target_bit_rate = bitrate;
            }
            RateControlMode::ConstantBitrate(bitrate) => {
                self.cfg.rate_control_mode = 2;
                self.cfg.target_bit_rate = bitrate;
            }
        }

        self
    }

    /// Sets the maximum bitrate in bits per second. Only applicable when using
    /// [`RateControlMode::ConstantQp`] or
    /// [`RateControlMode::ConstantRateFactor`].
    pub fn max_bitrate(mut self, max_bitrate: u32) -> Self {
        self.cfg.max_bit_rate = max_bitrate;
        self
    }

    /// Sets the range of QP values allowed when using
    /// [`RateControlMode::VariableBitrate`]. The values must be in the range
    /// 0-63.
    pub fn qp_range(mut self, min_qp: u32, max_qp: u32) -> Self {
        self.cfg.min_qp_allowed = min_qp;
        self.cfg.max_qp_allowed = max_qp;
        self
    }

    /// Sets the minimum bitrate to be used for a single GOP, as a percentage of
    /// the target bitrate. The values must be in the range 0-100. Only
    /// applicable when using [`RateControlMode::VariableBitrate`].
    pub fn bitrate_section_percentage(mut self, min: u32, max: u32) -> Self {
        self.cfg.vbr_min_section_pct = min;
        self.cfg.vbr_max_section_pct = max;
        self
    }

    /// Sets the  under/overshoot percentage for
    /// [`RateControlMode::VariableBitrate`] and
    /// [`RateControlMode::ConstantBitrate`] modes. The values must be in the
    /// range 0-100.
    pub fn bitrate_under_over_shoot_percentage(mut self, under: u32, over: u32) -> Self {
        self.cfg.under_shoot_pct = under;
        self.cfg.over_shoot_pct = over;
        self
    }

    /// Sets the max bitrate overshoot percentage for
    /// [`RateControlMode::ConstantRateFactor`]. The value must be in the range
    /// 0-100.
    pub fn max_bitrate_overshoot_percentage(mut self, max: u32) -> Self {
        self.cfg.mbr_over_shoot_pct = max;
        self
    }

    /// Sets the starting buffer level, in milliseconds. Influences decoding
    /// latency. The value must be in the range 20-10000. Only applicable to
    /// [`RateControlMode::ConstantBitrate`].
    pub fn starting_buffer_level(mut self, buffer_level: i64) -> Self {
        self.cfg.starting_buffer_level_ms = buffer_level;
        self
    }

    /// Sets the optimal buffer level, in milliseconds. Influences decoding
    /// latency. The value must be in the range 20-10000. Only applicable to
    /// [`RateControlMode::ConstantBitrate`].
    pub fn optimal_buffer_level(mut self, buffer_level: i64) -> Self {
        self.cfg.optimal_buffer_level_ms = buffer_level;
        self
    }

    /// Sets the maximum buffer size, in milliseconds. Influences decoding
    /// latency. The value must be in the range 20-10000. Only applicable to
    /// [`RateControlMode::ConstantBitrate`].
    pub fn maximum_buffer_size(mut self, buffer_level: i64) -> Self {
        self.cfg.maximum_buffer_size_ms = buffer_level;
        self
    }

    /// Enables the deblocking loop filter.
    pub fn enable_dlf(mut self, v: bool) -> Self {
        self.cfg.enable_dlf_flag = v.into();
        self
    }

    /// Enables film grain synthesis. A value of 0 disables the feature, while a
    /// value between 1-50 sets the strength.
    pub fn enable_film_grain_synthesis(mut self, strength: u32) -> Self {
        self.cfg.film_grain_denoise_strength = strength;
        self
    }

    /// Enables film grain denoising of the encoded output. If disabled, the
    /// film grain signaling is sent in the frame header.
    pub fn enable_film_grain_apply_denoise(mut self, apply: bool) -> Self {
        self.cfg.film_grain_denoise_apply = apply.into();
        self
    }

    /// Enables the constrained directional enhancement filter.
    pub fn enable_cdef(mut self, strength: CdefLevel) -> Self {
        self.cfg.cdef_level = match strength {
            CdefLevel::Off => 0,
            CdefLevel::Auto => -1,
            CdefLevel::Enable(strength) => strength as i32,
        };
        self
    }

    /// Enables restoration filtering. A value of none implies auto selection.
    pub fn enable_restoration_filtering(mut self, enable: Option<bool>) -> Self {
        self.cfg.enable_restoration_filtering = match enable {
            Some(true) => 1,
            Some(false) => 0,
            None => -1,
        };

        self
    }

    /// Enables motion field motion vector control.
    pub fn enable_motion_field_motion_vector_control(mut self, enable: Option<bool>) -> Self {
        self.cfg.enable_mfmv = match enable {
            Some(true) => 1,
            Some(false) => 0,
            None => -1,
        };

        self
    }

    /// Enables scene change detection.
    pub fn enable_scene_change_detection(mut self, v: bool) -> Self {
        self.cfg.scene_change_detection = v.into();
        self
    }

    /// Configures the encoder to disallow motion vectors that point outside the frame.
    pub fn restricted_motion_vector(mut self, v: bool) -> Self {
        self.cfg.restricted_motion_vector = v.into();
        self
    }

    /// Enables multi-tiling.
    pub fn tiling_mode(mut self, enable: TilingMode) -> Self {
        match enable {
            TilingMode::Single => {
                self.cfg.tile_columns = 0;
                self.cfg.tile_rows = 0;
            }
            TilingMode::Multi { columns, rows } => {
                self.cfg.tile_columns = columns as i32;
                self.cfg.tile_rows = rows as i32;
            }
        }

        self
    }

    /// Sets the look-ahead distance.
    pub fn look_ahead_distance(mut self, distance: u32) -> Self {
        self.cfg.look_ahead_distance = distance;
        self
    }

    /// Enables the Temporal Dependency Model (TPL for short).
    pub fn enable_tpl(mut self, v: bool) -> Self {
        self.cfg.enable_tpl_la = v.into();
        self
    }

    /// Sets the recode loop level.
    pub fn recode_level(mut self, level: RecodeLevel) -> Self {
        self.cfg.recode_loop = match level {
            RecodeLevel::Disable => 0,
            RecodeLevel::EnableForKeyframesAndBandwidth => 1,
            RecodeLevel::EnableForKeyframes => 2,
            RecodeLevel::EnableAll => 3,
            RecodeLevel::Auto => 4,
        };

        self
    }

    /// Enables screen content mode.
    pub fn enable_screen_content_mode(mut self, v: bool) -> Self {
        self.cfg.screen_content_mode = v.into();
        self
    }

    /// Enables the use of alt-ref (temporally filtered) frames.
    pub fn enable_tf(mut self, v: bool) -> Self {
        self.cfg.enable_tf = v.into();
        self
    }

    /// Sets the tuning metric.
    pub fn tune(mut self, tune: Tune) -> Self {
        self.cfg.tune = match tune {
            Tune::Vq => 0,
            Tune::Psnr => 1,
            Tune::Ssim => 2,
        };
        self
    }

    /// Enables fast-decode mode.
    pub fn enable_fast_decode(mut self, v: bool) -> Self {
        self.cfg.fast_decode = v.into();
        self
    }

    /// Configures the use of switch frames.
    pub fn switch_frame_insertion(mut self, mode: SwitchFrameInsertion) -> Self {
        match mode {
            SwitchFrameInsertion::Disabled => {
                self.cfg.sframe_dist = 0;
            }
            SwitchFrameInsertion::Strict(interval) => {
                self.cfg.sframe_dist = interval as i32;
                self.cfg.sframe_mode = EbSFrameMode_SFRAME_NEAREST_BASE;
            }
            SwitchFrameInsertion::Nearest(interval) => {
                self.cfg.sframe_dist = interval as i32;
                self.cfg.sframe_mode = EbSFrameMode_SFRAME_NEAREST_BASE;
            }
        }

        self
    }

    /// Sets the ID for the channel, if multiple channels are used.
    pub fn channel_id(mut self, id: u32) -> Self {
        self.cfg.channel_id = id;
        self
    }

    /// Sets the number of active channels.
    pub fn active_channel_count(mut self, count: u32) -> Self {
        self.cfg.active_channel_count = count;
        self
    }

    /// Configures the number of logical processors to use.
    pub fn logical_processors(mut self, count: u32) -> Self {
        self.cfg.logical_processors = count;
        self
    }

    /// Configures the encoder to pin execution to the cores specified by [`Av1EncoderConfig::logical_processors`].
    pub fn enable_pinned_execution(mut self, v: bool) -> Self {
        self.cfg.pin_threads = v.into();
        self
    }

    /// Configures the target socket to use, for dual-socket systems.
    pub fn target_socket(mut self, socket: TargetSocket) -> Self {
        self.cfg.target_socket = match socket {
            TargetSocket::Both => -1,
            TargetSocket::First => 0,
            TargetSocket::Second => 1,
        };

        self
    }

    /// Configures the enabled assembly instruction sets.
    pub fn cpu_flags(mut self, flags: CpuFlags) -> Self {
        self.cfg.use_cpu_flags = flags.bits();
        self
    }
}
