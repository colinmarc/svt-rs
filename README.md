# svt-rs

This repo contains rust bindings for the SVT (Scalable Video Technology) family of permissively-licensed and massively parallel video encoders. 

Right now, two encoders are included: [SVT-HEVC](https://github.com/OpenVisualCloud/SVT-HEVC) and [SVT-AV1](https://gitlab.com/AOMediaCodec/SVT-AV1). Support for the former is under the `hevc` feature, while support for the latter is under the `av1` feature.

Two `sys` crates are also provided. Both (currently only) support static linking. With the `log` feature, logs can be redirected through the [log](https://docs.rs/log/latest/log/
) crate.