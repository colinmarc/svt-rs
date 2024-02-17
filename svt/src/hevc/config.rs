use svt_hevc_sys::*;

use crate::{Error, SubsamplingFormat};

use super::{result, HevcEncoder, LibraryHandle};

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
    Closed(i32),
}

/// The prediction structure for each GOP.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PredictionStructure {
    /// Forward prediction only, using P frames.
    LowDelayP,
    /// Forward prediction only, using B frames.
    LowDelayB,
    /// Bidirectional prediction.
    RandomAccess,
}

/// When using [PredictionStructure::RandomAccess], the frame type to use in the base layer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BaseLayerSwitchMode {
    /// Use B frames.
    B,
    /// Use P frames.
    P,
}

/// The tiling mode to use.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TilingMode {
    /// Use a single tile.
    Single,
    /// Use multiple tiles.
    Multi {
        /// The number of columns.
        columns: u8,
        /// The number of rows.
        rows: u8,
    },
}

/// The rate control mode to use.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RateControlMode {
    /// Use a constant quantization parameter.
    ConstantQp,
    /// Use variable bitrate.
    VariableBitrate,
}

/// Whether to use ASM optimizations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AsmType {
    /// Do not use ASM optimizations.
    None,
    /// Auto-select the highest assembly instruction set supported.
    Auto,
}

/// The HEVC profile.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HevcProfile {
    /// Main profile.
    Main,
    /// Main 10 profile.
    Main10,
}

/// Hevc decoder tier.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HevcTier {
    /// Main, for most applications.
    Main,
    /// High, for demanding applications.
    High,
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
/// <https://github.com/OpenVisualCloud/SVT-HEVC/blob/master/Docs/svt-hevc_encoder_user_guide.md>
pub struct HevcEncoderConfig {
    handle: LibraryHandle,
    cfg: EB_H265_ENC_CONFIGURATION,
}

impl Default for HevcEncoderConfig {
    fn default() -> Self {
        unsafe {
            let mut handle = std::ptr::null_mut();
            let mut cfg = std::mem::zeroed();

            let res = EbInitHandle(&mut handle, std::ptr::null_mut(), &mut cfg);
            assert_eq!(0, res);

            HevcEncoderConfig {
                handle: LibraryHandle(handle),
                cfg,
            }
        }
    }
}

impl std::fmt::Debug for HevcEncoderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EncoderConfig")
            .field(&self.handle.as_ptr())
            .finish()
    }
}

impl HevcEncoderConfig {
    /// Creates a new encoder from the config.
    pub fn create_encoder(
        mut self,
        width: u32,
        height: u32,
        subsampling_format: SubsamplingFormat,
    ) -> Result<HevcEncoder, Error> {
        // Set the frame size.
        self.cfg.sourceWidth = width;
        self.cfg.sourceHeight = height;
        self.cfg.encoderColorFormat = match subsampling_format {
            SubsamplingFormat::Yuv400 => 0,
            SubsamplingFormat::Yuv420 => 1,
            SubsamplingFormat::Yuv422 => 2,
            SubsamplingFormat::Yuv444 => 3,
        };

        // Copy config parameters onto the encoder handle.
        unsafe { result(EbH265EncSetParameter(self.handle.as_ptr(), &mut self.cfg))? }

        // Create the encoder.
        unsafe { result(EbInitEncoder(self.handle.as_ptr()))? }

        Ok(HevcEncoder {
            handle: self.handle,
            intra_refresh_type: match self.cfg.intraRefreshType {
                -1 => IntraRefreshType::Open,
                gop_size => IntraRefreshType::Closed(gop_size),
            },
            subsampling_format: match self.cfg.encoderColorFormat {
                0 => SubsamplingFormat::Yuv400,
                1 => SubsamplingFormat::Yuv420,
                2 => SubsamplingFormat::Yuv422,
                3 => SubsamplingFormat::Yuv444,
                _ => unreachable!(),
            },
        })
    }

    /// Sets the encoder preset, from 0-11, with 0 being the highest quality and
    /// 11 the highest density.
    pub fn preset(mut self, preset: u8) -> Self {
        self.cfg.encMode = preset;
        self
    }

    /// Sets the intra refresh period.
    pub fn intra_period_length(mut self, intra_period_length: IntraPeriod) -> Self {
        self.cfg.intraPeriodLength = match intra_period_length {
            IntraPeriod::Auto => -2,
            IntraPeriod::None => -1,
            IntraPeriod::Fixed(frames) => frames as i32,
        };

        self
    }

    /// Sets the intra refresh type.
    pub fn intra_refresh_type(mut self, intra_refresh_type: IntraRefreshType) -> Self {
        self.cfg.intraRefreshType = match intra_refresh_type {
            IntraRefreshType::Open => -1,
            IntraRefreshType::Closed(gop_size) => gop_size,
        };

        self
    }

    /// Sets the number of hierarchical layers to use in each GOP.
    pub fn hierarchical_levels(mut self, hierarchical_levels: u32) -> Self {
        self.cfg.hierarchicalLevels = hierarchical_levels;
        self
    }

    /// Configures the prediction structure for each GOP.
    pub fn pred_structure(mut self, pred_structure: PredictionStructure) -> Self {
        self.cfg.predStructure = match pred_structure {
            PredictionStructure::LowDelayP => 0,
            PredictionStructure::LowDelayB => 1,
            PredictionStructure::RandomAccess => 2,
        };

        self
    }

    /// Changes the type of frame used for the base layer in [PredictionStructure::RandomAccess] mode.
    pub fn base_layer_switch_mode(mut self, base_layer_switch_mode: BaseLayerSwitchMode) -> Self {
        self.cfg.baseLayerSwitchMode = match base_layer_switch_mode {
            BaseLayerSwitchMode::B => 0,
            BaseLayerSwitchMode::P => 1,
        };

        self
    }

    /// Sets the input framerate.
    pub fn framerate(mut self, numerator: u32, denominator: u32) -> Self {
        self.cfg.frameRateNumerator = numerator as i32;
        self.cfg.frameRateDenominator = denominator as i32;
        self
    }

    /// Sets the input bit depth (8 or 10).
    pub fn encoder_bit_depth(mut self, bit_depth: u32) -> Self {
        self.cfg.encoderBitDepth = bit_depth;
        self
    }

    /// Configures the encoder to expect a special format for the input, where
    /// the extra two bits in 10-bit are packed.
    pub fn compressed_ten_bit_format(mut self, v: bool) -> Self {
        self.cfg.compressedTenBitFormat = v as u32;
        self
    }

    /// Enables automatic subjective bit rate reduction.
    pub fn enable_auto_bit_rate_reduction(mut self, v: bool) -> Self {
        self.cfg.bitRateReduction = v as u8;
        self
    }

    /// Enables the use of a sharpness algorithm. Only available for 4k or 8k resolutions.
    pub fn enable_auto_improve_sharpness(mut self, v: bool) -> Self {
        self.cfg.improveSharpness = v as u8;
        self
    }

    /// Enables SEI messages with interlaced signaling.
    pub fn enable_interlaced_video(mut self, v: bool) -> Self {
        self.cfg.interlacedVideo = v as u8;
        self
    }

    /// Sets the target QP for [RateControlMode::ConstantQp].
    pub fn qp(mut self, qp: u32) -> Self {
        self.cfg.qp = qp;
        self
    }

    /// Enables multi-tile mode.
    pub fn tiling(mut self, tiling_mode: TilingMode) -> Self {
        match tiling_mode {
            TilingMode::Single => {
                self.cfg.tileSliceMode = 0;
            }
            TilingMode::Multi { columns, rows } => {
                self.cfg.tileColumnCount = columns;
                self.cfg.tileRowCount = rows;
                self.cfg.tileSliceMode = 1;
            }
        }

        self
    }

    /// Disables deblocking loop filtering.
    pub fn disable_dlf(mut self, v: bool) -> Self {
        self.cfg.disableDlfFlag = v as u8;
        self
    }

    /// Enables sample adaptive filtering.
    pub fn enable_sao(mut self, v: bool) -> Self {
        self.cfg.enableSaoFlag = v as u8;
        self
    }

    /// Enables the use of default motion estimation parameters.
    pub fn use_default_me_hme(mut self, v: bool) -> Self {
        self.cfg.useDefaultMeHme = v as u8;
        self
    }

    /// Enables the use of hierarchical motion estimation.
    pub fn enable_hme(mut self, v: bool) -> Self {
        self.cfg.enableHmeFlag = v as u8;
        self
    }

    /// Sets the search area width for motion estimation. If not set and
    /// [HevcEncoderConfig::use_default_me_hme] is true (the default), the width
    /// will depend on the input resolution.
    pub fn search_area_width(mut self, width: u32) -> Self {
        self.cfg.searchAreaWidth = width;
        self
    }

    /// Sets the search area height for motion estimation. If not set and
    /// [HevcEncoderConfig::use_default_me_hme] is true (the default), the height
    /// will depend on the input resolution.
    pub fn search_area_height(mut self, height: u32) -> Self {
        self.cfg.searchAreaHeight = height;
        self
    }

    /// Enables constrained intra.
    pub fn enable_constrained_intra(mut self, pred: bool) -> Self {
        self.cfg.constrainedIntra = pred as u8;
        self
    }

    /// Sets the rate control mode.
    pub fn rate_control_mode(mut self, rate_control_mode: RateControlMode) -> Self {
        self.cfg.rateControlMode = match rate_control_mode {
            RateControlMode::ConstantQp => 0,
            RateControlMode::VariableBitrate => 1,
        };

        self
    }

    /// Enables scene change detection.
    pub fn enable_scene_change_detection(mut self, v: bool) -> Self {
        self.cfg.sceneChangeDetection = v as u32;
        self
    }

    /// Sets the look-ahead distance.
    pub fn look_ahead_distance(mut self, distance: u32) -> Self {
        self.cfg.lookAheadDistance = distance;
        self
    }

    /// Sets the target bitrate for the [RateControlMode::VariableBitrate] mode.
    pub fn target_bitrate(mut self, bitrate: u32) -> Self {
        self.cfg.targetBitRate = bitrate;
        self
    }

    /// Sets the maximum QP for the [RateControlMode::VariableBitrate] mode.
    pub fn max_qp_allowed(mut self, qp: u32) -> Self {
        self.cfg.maxQpAllowed = qp;
        self
    }

    /// Sets the minimum QP for the [RateControlMode::VariableBitrate] mode.
    pub fn min_qp_allowed(mut self, qp: u32) -> Self {
        self.cfg.minQpAllowed = qp;
        self
    }

    /// Enables generation of VPS, SPS, and PPS NAL units.
    pub fn code_vps_sps_pps(mut self, v: bool) -> Self {
        self.cfg.codeVpsSpsPps = v as u8;
        self
    }

    /// Enables generation of EOS NAL units.
    pub fn code_eos(mut self, v: bool) -> Self {
        self.cfg.codeEosNal = v as u8;
        self
    }

    /// Enables generation of VUI NAL units.
    pub fn code_vui(mut self, v: bool) -> Self {
        self.cfg.videoUsabilityInfo = v as u32;
        self
    }

    /// Configures the encoder to expect input in the BT2020 color space. Only applicable for 10-bit input. Requries [HevcEncoderConfig::code_vui] to be enabled.
    pub fn hdr_input(mut self, v: bool) -> Self {
        self.cfg.highDynamicRangeInput = v as u32;
        self
    }

    /// Enables generation of access unit delimiters.
    pub fn code_access_unit_delimiters(mut self, v: bool) -> Self {
        self.cfg.accessUnitDelimiter = v as u32;
        self
    }

    /// Enables generation of buffering period SEI NAL units.
    pub fn code_buffering_period_sei(mut self, v: bool) -> Self {
        self.cfg.bufferingPeriodSEI = v as u32;
        self
    }

    /// Enables generation of picture timing SEI NAL units.
    pub fn code_picture_timing_sei(mut self, v: bool) -> Self {
        self.cfg.pictureTimingSEI = v as u32;
        self
    }

    /// Enables generation of user data SEI NAL units for registered users.
    pub fn registered_user_data_sei(mut self, v: bool) -> Self {
        self.cfg.registeredUserDataSeiFlag = v as u32;
        self
    }

    /// Enables generation of user data SEI NAL units for unregistered users.
    pub fn unregistered_user_data_sei(mut self, v: bool) -> Self {
        self.cfg.unregisteredUserDataSeiFlag = v as u32;
        self
    }

    /// Enables generation of recovery point SEI NAL units.
    pub fn recovery_point_sei(mut self, v: bool) -> Self {
        self.cfg.recoveryPointSeiFlag = v as u32;
        self
    }

    /// Enables insertion of temporal IDs in NAL units.
    pub fn enable_teporal_id(mut self, v: bool) -> Self {
        self.cfg.enableTemporalId = v as u32;
        self
    }

    /// Sets the HEVC profile.
    pub fn profile(mut self, profile: HevcProfile) -> Self {
        self.cfg.profile = match profile {
            HevcProfile::Main => 0,
            HevcProfile::Main10 => 1,
        };

        self
    }

    /// Enables the usage of high-tier HEVC features.
    pub fn tier(mut self, tier: HevcTier) -> Self {
        self.cfg.tier = match tier {
            HevcTier::Main => 0,
            HevcTier::High => 1,
        };

        self
    }

    /// Sets the IDC level. A value of 0 configures the encoder to auto-detect
    /// the level. Any other value represents the level multiplied by ten - for
    /// example, 31 for level 3.1.
    pub fn level(mut self, level: u32) -> Self {
        self.cfg.level = level;
        self
    }

    /// Enables VPS timing info.
    pub fn enable_fps_in_vps(mut self, v: bool) -> Self {
        self.cfg.fpsInVps = v as u8;
        self
    }

    /// Sets the VBV maximum rate in bits/second. Only used with [RateControlMode::VariableBitrate].
    pub fn vbv_max_rate(mut self, rate: u32) -> Self {
        self.cfg.vbvMaxrate = rate;
        self
    }

    /// Sets the VBV buffer size in bits/second. Only used with [RateControlMode::VariableBitrate].
    pub fn vbv_buf_size(mut self, size: u32) -> Self {
        self.cfg.vbvBufsize = size;
        self
    }

    /// Sets the initial VBV buffer fullness. Only used with [RateControlMode::VariableBitrate].
    pub fn vbv_buf_init(mut self, init: u64) -> Self {
        self.cfg.vbvBufInit = init;
        self
    }

    /// Enables the Hypothetical Reference Decoder flag.
    pub fn enable_hrd(mut self, v: bool) -> Self {
        self.cfg.hrdFlag = v as u32;
        self
    }

    /// Sets the ID for the channel, if multiple channels are used.
    pub fn channel_id(mut self, id: u32) -> Self {
        self.cfg.channelId = id;
        self
    }

    /// Sets the number of active channels.
    pub fn active_channel_count(mut self, count: u32) -> Self {
        self.cfg.activeChannelCount = count;
        self
    }

    /// Configures the number of logical processors to use.
    pub fn logical_processors(mut self, count: u32) -> Self {
        self.cfg.logicalProcessors = count;
        self
    }

    /// Configures the first logical processor to use.
    pub fn first_logical_processor(mut self, count: u32) -> Self {
        self.cfg.firstLogicalProcessor = count;
        self
    }

    /// Configures the target socket to use, for dual-socket systems.
    pub fn target_socket(mut self, socket: TargetSocket) -> Self {
        self.cfg.targetSocket = match socket {
            TargetSocket::Both => -1,
            TargetSocket::First => 0,
            TargetSocket::Second => 1,
        };

        self
    }

    /// On linux, attempts to enable real-time priority on the encoding thread(s).
    pub fn switch_threads_to_rt(mut self, v: bool) -> Self {
        self.cfg.switchThreadsToRtPriority = v as u8;
        self
    }

    /// Sets the number of worker threads to create.
    pub fn thread_count(mut self, count: u32) -> Self {
        self.cfg.threadCount = count;
        self
    }

    /// Configures which assembly instruction set to use.
    pub fn asm_type(mut self, asm_type: AsmType) -> Self {
        self.cfg.asmType = match asm_type {
            AsmType::None => 0,
            AsmType::Auto => 1,
        };

        self
    }

    /// Enables speed control, which dynamically adjusts the preset to match
    /// [HevcEncoderConfig::framerate].
    pub fn enable_speed_control(mut self, speed_control: bool) -> Self {
        self.cfg.speedControlFlag = speed_control as u32;
        self
    }

    /// Configures the rate at which input frames will be injected.
    pub fn injector_framerate(mut self, hz: u32) -> Self {
        self.cfg.injectorFrameRate = hz as i32;
        self
    }

    /// Configures the encoder to allow motion vectors to point outside the frame.
    pub fn unrestricted_motion_vector(mut self, v: bool) -> Self {
        self.cfg.unrestrictedMotionVector = v as u8;
        self
    }
}
