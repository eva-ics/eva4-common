use crate::{EResult, Error, Value};
use binrw::prelude::*;
use gst::{Buffer, BufferFlags, BufferRef, Caps, CapsRef};
use gst_video::VideoInfo;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator as _;
use strum::{Display, EnumIter, EnumString};

#[binrw]
#[brw(little, magic = b"EVS")]
#[derive(Clone, Debug)]
pub struct FrameHeader {
    version: u8,
    format: u8,
    width: u16,
    height: u16,
    // bits
    // 0 - key frame
    // 1-7 - reserved
    flags: u8,
}

impl FrameHeader {
    pub const SIZE: usize = 7 + 3;
    pub fn new(format: VideoFormat, width: u16, height: u16) -> Self {
        FrameHeader {
            version: EVA_MULTIMEDIA_VERSION,
            format: format as u8,
            width,
            height,
            flags: 0,
        }
    }
    pub fn set_key_frame(&mut self) {
        self.flags |= 0b0000_0001;
    }
    pub fn from_slice(slice: &[u8]) -> Result<Self, binrw::Error> {
        let mut cursor = std::io::Cursor::new(slice);
        FrameHeader::read(&mut cursor)
    }
    pub fn try_from_caps(caps: &gst::Caps) -> EResult<Self> {
        let caps_ref = caps.as_ref();
        Self::try_from_caps_ref(caps_ref)
    }
    pub fn try_from_caps_ref(caps_ref: &CapsRef) -> EResult<Self> {
        let s = caps_ref.structure(0).ok_or_else(|| {
            Error::invalid_data("No structure found in the provided GStreamer caps")
        })?;
        let codec: VideoFormat = s.name().parse().map_err(Error::invalid_data)?;
        let info = VideoInfo::from_caps(caps_ref).map_err(Error::invalid_data)?;
        let width = info.width().try_into()?;
        let height = info.height().try_into()?;
        Ok(Self::new(codec, width, height))
    }
    pub fn try_to_caps(&self) -> EResult<gst::Caps> {
        let format = self.format()?;
        Ok(format.into_caps_with_dimensions(self.width.into(), self.height.into()))
    }
    /// # Panics
    ///
    /// Will panic if the memory write operation fails.
    pub fn into_vec(self, vec_size: usize) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(vec_size));
        self.write(&mut buf).expect("Failed to write frame header");
        buf.into_inner()
    }
    pub fn format(&self) -> EResult<VideoFormat> {
        VideoFormat::try_from(self.format)
    }
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn height(&self) -> u16 {
        self.height
    }
    pub fn is_key_frame(&self) -> bool {
        self.flags & 0b0000_0001 != 0
    }
    pub fn is_version_valid(&self) -> bool {
        self.version == EVA_MULTIMEDIA_VERSION
    }
}

pub const EVA_MULTIMEDIA_VERSION: u8 = 1;

#[derive(
    Serialize,
    Deserialize,
    EnumString,
    EnumIter,
    Display,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
)]
#[repr(u8)]
pub enum VideoFormat {
    #[strum(serialize = "video/x-raw")]
    #[serde(rename = "raw")]
    Raw = 0,
    #[strum(serialize = "video/x-h264")]
    #[serde(rename = "h264")]
    H264 = 10,
    #[strum(serialize = "video/x-h265")]
    #[serde(rename = "h265")]
    H265 = 11,
    #[strum(serialize = "video/x-vp8")]
    #[serde(rename = "vp8")]
    VP8 = 12,
    #[strum(serialize = "video/x-vp9")]
    #[serde(rename = "vp9")]
    VP9 = 13,
    #[strum(serialize = "video/x-av1")]
    #[serde(rename = "av1")]
    AV1 = 14,
}

impl TryFrom<u8> for VideoFormat {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(VideoFormat::Raw),
            10 => Ok(VideoFormat::H264),
            11 => Ok(VideoFormat::H265),
            12 => Ok(VideoFormat::VP8),
            13 => Ok(VideoFormat::VP9),
            14 => Ok(VideoFormat::AV1),
            _ => Err(Error::invalid_data(format!(
                "Unknown video format code: {}",
                value
            ))),
        }
    }
}

impl VideoFormat {
    pub fn into_caps(self) -> gst::Caps {
        if self == VideoFormat::Raw {
            return Caps::builder(self.to_string())
                .field("format", "RGB")
                .build();
        }
        Caps::builder(self.to_string()).build()
    }
    pub fn into_caps_with_dimensions(self, width: u32, height: u32) -> gst::Caps {
        if self == VideoFormat::Raw {
            return Caps::builder(self.to_string())
                .field("format", "RGB")
                .field("width", i32::try_from(width).unwrap_or(i32::MAX))
                .field("height", i32::try_from(height).unwrap_or(i32::MAX))
                .build();
        }
        Caps::builder(self.to_string()).build()
    }
    pub fn all_caps() -> gst::Caps {
        let mut caps = Caps::new_empty();
        let caps_mut = caps.make_mut();

        for format in VideoFormat::iter() {
            caps_mut.append(format.into_caps());
        }
        caps
    }
}

impl Value {
    pub fn try_from_gstreamer_buffer(header: &FrameHeader, buffer: &Buffer) -> EResult<Value> {
        Self::try_from_gstreamer_buffer_ref(header, buffer.as_ref())
    }
    pub fn try_from_gstreamer_buffer_ref(
        header: &FrameHeader,
        buffer_ref: &BufferRef,
    ) -> EResult<Value> {
        let mut header = header.clone();
        let flags = buffer_ref.flags();
        let is_key = !flags.contains(BufferFlags::DELTA_UNIT);
        if is_key {
            header.set_key_frame();
        }
        let mut frame_buf = header.into_vec(buffer_ref.size());
        frame_buf.extend(
            buffer_ref
                .map_readable()
                .map_err(Error::invalid_data)?
                .as_slice(),
        );
        Ok(Value::Bytes(frame_buf))
    }
}
