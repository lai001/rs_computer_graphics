use crate::{
    hw::{
        find_hw_pix_fmt, get_available_hwdevice_types, get_hw_format, hw_decoder_init, MyUserData,
    },
    time_range::TimeRangeRational,
};
use ffmpeg_next::ffi::{
    av_hwframe_transfer_data, av_seek_frame, AVCodec, AVCodecContext, AVHWDeviceType,
    AVPixelFormat, AVSEEK_FLAG_BACKWARD,
};
use rs_foundation::TimeRange;
use std::collections::HashMap;

struct HWSection {
    user_data: *mut MyUserData,
    expect_hw_type: AVHWDeviceType,
    release_closure: Option<Box<dyn FnMut() -> ()>>,
}

impl HWSection {
    fn new(
        expect_hw_type: AVHWDeviceType,
        codec: *const AVCodec,
    ) -> crate::error::Result<HWSection> {
        if codec.is_null() {
            return Err(crate::error::Error::Other(format!("Codec is null")));
        }
        let mut hw_pixel_formats = HashMap::<AVHWDeviceType, Vec<AVPixelFormat>>::new();
        for device_type in get_available_hwdevice_types() {
            for pix_fmt in unsafe { find_hw_pix_fmt(codec, device_type) } {
                hw_pixel_formats
                    .entry(device_type)
                    .or_default()
                    .push(pix_fmt);
            }
        }

        assert!(hw_pixel_formats.contains_key(&expect_hw_type));
        let user_data = MyUserData {
            hw_pix_fmt: *hw_pixel_formats
                .get(&expect_hw_type)
                .ok_or(crate::error::Error::Other(format!("Not support")))?
                .first()
                .ok_or(crate::error::Error::Other(format!("Not support")))?,
        };
        let user_data = Box::new(user_data);
        let user_data = Box::into_raw(user_data);
        Ok(HWSection {
            user_data,
            expect_hw_type,
            release_closure: None,
        })
    }

    fn init(&mut self, codec_context: *mut AVCodecContext) {
        assert_ne!(codec_context, std::ptr::null_mut());
        unsafe {
            (*codec_context).opaque = std::mem::transmute(self.user_data);
            (*codec_context).get_format = Some(get_hw_format);
            let closure = hw_decoder_init(codec_context, self.expect_hw_type);
            self.release_closure = Some(Box::new(closure));
        }
    }

    fn get_hw_pix_fmt(&mut self) -> AVPixelFormat {
        let user_data = unsafe { self.user_data.as_ref() }.expect(&format!("Should not be null"));
        user_data.hw_pix_fmt.clone()
    }
}

impl Drop for HWSection {
    fn drop(&mut self) {
        assert_ne!(self.user_data, std::ptr::null_mut());
        if let Some(closure) = &mut self.release_closure {
            closure();
        }
        unsafe {
            let _ = Box::from_raw(self.user_data);
        };
    }
}

pub struct VideoFrame {
    pub time_range_rational: TimeRangeRational,
    pub image: image::RgbaImage,
    pub pict_type: ffmpeg_next::ffi::AVPictureType,
}

impl VideoFrame {
    pub fn get_time_range_second(&self) -> TimeRange {
        self.time_range_rational.get_time_range_second()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EVideoDecoderType {
    Software,
    Hardware,
}

pub struct VideoFrameExtractor {
    format_input: ffmpeg_next::format::context::Input,
    video_decoder: ffmpeg_next::codec::decoder::Video,
    video_stream_index: usize,
    time_base: ffmpeg_next::Rational,
    video_decoder_type: EVideoDecoderType,
    hw_section: Option<HWSection>,
    scaler: Option<ffmpeg_next::software::scaling::Context>,
    decoded_video_frame: ffmpeg_next::frame::Video,
    rescale_video_frame: ffmpeg_next::frame::Video,
    hwframe: Option<ffmpeg_next::frame::Video>,
}

impl VideoFrameExtractor {
    pub fn new(
        filepath: &str,
        video_decoder_type: Option<EVideoDecoderType>,
    ) -> crate::error::Result<VideoFrameExtractor> {
        let format_input = ffmpeg_next::format::input(&filepath.to_owned())
            .map_err(|err| crate::error::Error::FFMpeg(err))?;
        let input_stream = format_input
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(crate::error::Error::Other(format!("No video stream")))?;

        let time_base = input_stream.time_base();
        let video_stream_index = input_stream.index();
        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
                .map_err(|err| crate::error::Error::FFMpeg(err))?;
        let mut video_decoder = context_decoder
            .decoder()
            .video()
            .map_err(|err| crate::error::Error::FFMpeg(err))?;
        unsafe { (*video_decoder.as_mut_ptr()).pkt_timebase = time_base.into() };
        let video_decoder_type = video_decoder_type.unwrap_or(EVideoDecoderType::Software);

        let mut video_player_item = VideoFrameExtractor {
            format_input,
            video_decoder,
            video_stream_index,
            time_base,
            video_decoder_type,
            hw_section: None,
            scaler: None,
            decoded_video_frame: ffmpeg_next::frame::Video::empty(),
            rescale_video_frame: ffmpeg_next::frame::Video::empty(),
            hwframe: None,
        };
        match video_decoder_type {
            EVideoDecoderType::Software => {}
            EVideoDecoderType::Hardware => {
                let expect_hw_type = AVHWDeviceType::AV_HWDEVICE_TYPE_CUDA;
                let codec = unsafe {
                    video_player_item
                        .video_decoder
                        .codec()
                        .ok_or(crate::error::Error::Other(format!("No video codec")))?
                        .as_ptr()
                };
                let mut hw_section_in = HWSection::new(expect_hw_type, codec)?;
                hw_section_in.init(unsafe { video_player_item.video_decoder.as_mut_ptr() });
                video_player_item.hw_section = Some(hw_section_in);
                video_player_item.hwframe = Some(ffmpeg_next::frame::Video::empty());
            }
        }
        video_player_item.scaler = Some(video_player_item.get_matched_scaler());

        video_player_item.seek(0.0);
        Ok(video_player_item)
    }

    pub fn get_stream_time_base(&self) -> ffmpeg_next::Rational {
        self.time_base
    }

    pub fn seek(&mut self, second: f32) {
        let seek_time: f32;
        {
            let stream = self.format_input.stream(self.video_stream_index).unwrap();
            seek_time = second * stream.time_base().denominator() as f32;
        }
        unsafe {
            match av_seek_frame(
                self.format_input.as_mut_ptr(),
                self.video_stream_index as i32,
                seek_time as i64,
                AVSEEK_FLAG_BACKWARD,
            ) {
                s if s >= 0 => {}
                e => {
                    let error = ffmpeg_next::Error::from(e);
                    log::error!("seek error: {}", error);
                }
            }
        };
    }

    fn get_matched_scaler(&mut self) -> ffmpeg_next::software::scaling::context::Context {
        let mut hw_pixel: Option<ffmpeg_next::format::Pixel> = None;
        if let Some(hw_section) = &mut self.hw_section {
            if hw_section.get_hw_pix_fmt() == AVPixelFormat::AV_PIX_FMT_CUDA {
                hw_pixel = Some(ffmpeg_next::format::Pixel::NV12);
            }
        }
        let format = match self.video_decoder_type {
            EVideoDecoderType::Software => self.video_decoder.format(),
            EVideoDecoderType::Hardware => hw_pixel.unwrap_or(ffmpeg_next::format::Pixel::NV12),
        };

        let scaler = ffmpeg_next::software::scaling::Context::get(
            format,
            self.video_decoder.width(),
            self.video_decoder.height(),
            ffmpeg_next::format::Pixel::RGBA,
            self.video_decoder.width(),
            self.video_decoder.height(),
            ffmpeg_next::software::scaling::Flags::BILINEAR,
        )
        .unwrap();
        scaler
    }

    fn find_next_packet(&mut self) -> Option<(ffmpeg_next::Stream, ffmpeg_next::Packet)> {
        let mut packet_iter = self.format_input.packets();
        loop {
            match packet_iter.next() {
                Some((stream, packet)) => {
                    if stream.index() == self.video_stream_index {
                        return Some((stream, packet));
                    }
                }
                None => {
                    break;
                }
            }
        }
        return None;
    }

    pub fn next_frames(&mut self) -> Option<Vec<VideoFrame>> {
        let Some((stream, packet)) = self.find_next_packet() else {
            return None;
        };

        let time_base = stream.time_base();
        let mut video_frames: Vec<VideoFrame> = vec![];
        self.video_decoder.send_packet(&packet).unwrap();
        while self
            .video_decoder
            .receive_frame(&mut self.decoded_video_frame)
            .is_ok()
        {
            let rescale_start_pts: i64;
            let rescale_end_pts: i64;
            let pict_type: ffmpeg_next::ffi::AVPictureType;
            unsafe {
                let pts = (*self.decoded_video_frame.as_mut_ptr()).pts;
                let duration = (*self.decoded_video_frame.as_mut_ptr()).duration;
                rescale_start_pts = pts;
                rescale_end_pts = rescale_start_pts + duration as i64;
                pict_type = (*self.decoded_video_frame.as_mut_ptr()).pict_type;
            }

            if self.video_decoder_type == EVideoDecoderType::Hardware {
                let state = unsafe {
                    av_hwframe_transfer_data(
                        self.hwframe.as_mut().unwrap().as_mut_ptr(),
                        self.decoded_video_frame.as_mut_ptr(),
                        0,
                    )
                };
                if state < 0 {
                    log::warn!("Error transferring the data to system memory");
                }
            }
            let decoded_video_frame = self.hwframe.as_ref().unwrap_or(&self.decoded_video_frame);
            match self
                .scaler
                .as_mut()
                .unwrap()
                .run(decoded_video_frame, &mut self.rescale_video_frame)
            {
                Ok(_) => {
                    let data = self.rescale_video_frame.data(0);
                    let image = image::RgbaImage::from_raw(
                        self.video_decoder.width(),
                        self.video_decoder.height(),
                        data.to_vec(),
                    )
                    .unwrap();
                    let video_frame = VideoFrame {
                        time_range_rational: TimeRangeRational {
                            start: ffmpeg_next::Rational(
                                rescale_start_pts as i32,
                                time_base.denominator(),
                            ),
                            end: ffmpeg_next::Rational(
                                rescale_end_pts as i32,
                                time_base.denominator(),
                            ),
                        },
                        image,
                        pict_type,
                    };
                    video_frames.push(video_frame);
                }
                Err(error) => log::warn!("{:?}", error),
            }
        }
        return Some(video_frames);
    }

    pub fn get_duration(&self) -> f32 {
        let video_stream = self
            .format_input
            .stream(self.video_stream_index)
            .expect("Should not be null");
        let time_base = video_stream.time_base();
        let duration = video_stream.duration();
        (duration as f32) * (time_base.numerator() as f32) / (time_base.denominator() as f32)
    }
}
