//! An example that encodes from y4m input on stdin, dumping an output stream to stdout.
//!
//! You can run it with, for example:
//!
//!     ffmpeg -loglevel error -i video.mp4 -f yuv4mpegpipe - | cargo run --example encode --features hevc | mpv -

use std::io::{self, Write};

use svt::{Encoder, Packet, Plane, YUVBuffer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = io::stdin();
    let mut y4m_decoder = y4m::decode(&mut stdin)?;

    let width = y4m_decoder.get_width() as u32;
    let height = y4m_decoder.get_height() as u32;
    let colorspace = match y4m_decoder.get_colorspace() {
        y4m::Colorspace::Cmono => svt::SubsamplingFormat::Yuv400,
        y4m::Colorspace::C420 | y4m::Colorspace::C420jpeg | y4m::Colorspace::C420mpeg2 => {
            svt::SubsamplingFormat::Yuv420
        }
        y4m::Colorspace::C422 => svt::SubsamplingFormat::Yuv422,
        y4m::Colorspace::C444 => svt::SubsamplingFormat::Yuv444,
        c => return Err(format!("unsupported colorspace: {:?}", c).into()),
    };

    let framerate = y4m_decoder.get_framerate();

    let mut buf = YUVBuffer::new(width, height, colorspace);

    let encoder = svt::av1::Av1EncoderConfig::default()
        .preset(8)
        .rate_control_mode(svt::av1::RateControlMode::ConstantRateFactor(30))
        .create_encoder(width, height, colorspace)?;

    let mut pts: i64 = 0;
    loop {
        match y4m_decoder.read_frame() {
            Ok(frame) => {
                // Copy into the input buffer.
                buf.as_mut_slice(Plane::Y)
                    .copy_from_slice(frame.get_y_plane());
                buf.as_mut_slice(Plane::U)
                    .copy_from_slice(frame.get_u_plane());
                buf.as_mut_slice(Plane::V)
                    .copy_from_slice(frame.get_v_plane());

                // Simulate presentation timestamp by incrementing for each
                // frame, based on the declared framerate.
                pts += 1000 * framerate.num as i64 / framerate.den as i64;

                encoder.send_picture(&buf, pts, false)?;

                while let Some(packet) = encoder.get_packet(false)? {
                    io::stdout().write_all(packet.as_bytes())?;
                }
            }
            Err(y4m::Error::EOF) => break,
            Err(e) => return Err(e.into()),
        }
    }

    // Flush the encoder.
    encoder.finish()?;
    while let Some(packet) = encoder.get_packet(true)? {
        io::stdout().write_all(packet.as_bytes())?;
        if packet.is_eos() {
            break;
        }
    }

    Ok(())
}
