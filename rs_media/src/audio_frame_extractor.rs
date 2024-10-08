use crate::{
    custom_io_input::{self, CleanClosure},
    media_stream::MediaStream,
    time_range::TimeRangeRational,
};
use ffmpeg_next::{
    ffi::{av_rescale_q_rnd, av_seek_frame, AVRational, AVRounding, AVSEEK_FLAG_BACKWARD},
    util::format,
};
use rs_core_audio::{
    audio_format::{AudioFormat, EAudioSampleType},
    audio_pcmbuffer::AudioPcmbuffer,
};
use rs_foundation::TimeRange;
use std::path::Path;

#[derive(Debug)]
pub struct AudioFrame {
    pub time_range_rational: TimeRangeRational,
    pub pcm_buffer: AudioPcmbuffer,
}

impl AudioFrame {
    pub fn get_time_range_second(&self) -> TimeRange {
        let start = self.time_range_rational.start.numerator() as f32
            / self.time_range_rational.start.denominator() as f32;
        let end = self.time_range_rational.end.numerator() as f32
            / self.time_range_rational.end.denominator() as f32;
        TimeRange { start, end }
    }
}

pub struct AudioFrameExtractor {
    format_input: ffmpeg_next::format::context::Input,
    audio_decoder: ffmpeg_next::codec::decoder::Audio,
    audio_stream_index: usize,
    time_base: ffmpeg_next::Rational,
    duration: f32,
    current_seek_time: f32,
    clean: Option<CleanClosure>,
}

impl AudioFrameExtractor {
    pub fn from_path(filepath: impl AsRef<Path>) -> crate::error::Result<AudioFrameExtractor> {
        let format_input = ffmpeg_next::format::input(&filepath)
            .map_err(|err| crate::error::Error::FFMpeg(err))?;
        Self::from_input(format_input, None)
    }

    pub fn from_data(data: Vec<u8>) -> crate::error::Result<AudioFrameExtractor> {
        let stream = MediaStream::new(data);
        let create_custom_ioresult = custom_io_input::input_with_custom_read_io(Box::new(stream))
            .map_err(|err| crate::error::Error::FFMpeg(err))?;
        Self::from_input(
            create_custom_ioresult.input,
            Some(create_custom_ioresult.clean),
        )
    }

    fn from_input(
        format_input: ffmpeg_next::format::context::Input,
        clean: Option<CleanClosure>,
    ) -> crate::error::Result<AudioFrameExtractor> {
        let input_stream = format_input
            .streams()
            .best(ffmpeg_next::media::Type::Audio)
            .ok_or(crate::error::Error::Other(format!(
                "No audio stream found."
            )))?;
        let time_base = input_stream.time_base();
        let audio_stream_index = input_stream.index();
        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
                .map_err(|err| crate::error::Error::FFMpeg(err))?;
        let mut audio_decoder = context_decoder
            .decoder()
            .audio()
            .map_err(|err| crate::error::Error::FFMpeg(err))?;
        unsafe { (*audio_decoder.as_mut_ptr()).pkt_timebase = time_base.into() };
        let duration = input_stream.duration() as f32 / input_stream.time_base().1 as f32;
        let mut item = AudioFrameExtractor {
            format_input,
            audio_decoder,
            audio_stream_index,
            time_base,
            duration,
            current_seek_time: 0.0,
            clean,
        };
        item.seek(0.0);
        Ok(item)
    }

    pub fn get_stream_time_base(&self) -> ffmpeg_next::Rational {
        self.time_base
    }

    pub fn get_duration(&self) -> f32 {
        self.duration
    }

    pub fn seek(&mut self, second: f32) {
        let seek_time: f32;
        {
            let stream = self.format_input.stream(self.audio_stream_index).unwrap();
            seek_time = second * stream.time_base().denominator() as f32;
        }
        unsafe {
            match av_seek_frame(
                self.format_input.as_mut_ptr(),
                self.audio_stream_index as i32,
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
        self.current_seek_time = seek_time;
    }

    fn find_next_packet(&mut self) -> Option<(ffmpeg_next::Stream, ffmpeg_next::Packet)> {
        let mut packet_iter = self.format_input.packets();
        loop {
            match packet_iter.next() {
                Some((stream, packet)) => {
                    if stream.index() == self.audio_stream_index {
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

    pub fn next_frames(&mut self) -> Option<Vec<AudioFrame>> {
        match self.find_next_packet() {
            Some((_, packet)) => {
                let mut audio_frames: Vec<AudioFrame> = vec![];
                self.audio_decoder.send_packet(&packet).unwrap();
                let mut decoded_audio_frame = ffmpeg_next::frame::Audio::empty();
                let mut resample_audio_frame = ffmpeg_next::frame::Audio::empty();
                while self
                    .audio_decoder
                    .receive_frame(&mut decoded_audio_frame)
                    .is_ok()
                {
                    let sample_rate: i32;
                    let rescale_start_pts: i64;
                    let rescale_end_pts: i64;
                    let nb_samples: i32;
                    unsafe {
                        sample_rate = (*decoded_audio_frame.as_mut_ptr()).sample_rate;
                        let pts = (*decoded_audio_frame.as_mut_ptr()).pts;
                        let duration = (*decoded_audio_frame.as_mut_ptr()).nb_samples;
                        nb_samples = duration;
                        rescale_start_pts = av_rescale_q_rnd(
                            pts,
                            self.time_base.into(),
                            AVRational {
                                num: 1,
                                den: sample_rate,
                            },
                            AVRounding::AV_ROUND_INF,
                        );
                        rescale_end_pts = rescale_start_pts + duration as i64;
                    }

                    let mut resample = ffmpeg_next::software::resampling::context::Context::get(
                        decoded_audio_frame.format(),
                        decoded_audio_frame.channel_layout(),
                        sample_rate as u32,
                        format::Sample::F32(ffmpeg_next::format::sample::Type::Planar),
                        ffmpeg_next::ChannelLayout::STEREO,
                        sample_rate as u32,
                    )
                    .unwrap();
                    resample
                        .run(&decoded_audio_frame, &mut resample_audio_frame)
                        .unwrap();

                    let audio_format = AudioFormat::from(
                        sample_rate as u32,
                        resample_audio_frame.channel_layout().channels() as u32,
                        EAudioSampleType::Float32,
                        true,
                    );

                    let mut pcm_buffer = AudioPcmbuffer::from(audio_format, nb_samples as usize);

                    for channel in 0..resample_audio_frame.channel_layout().channels() {
                        let buffer = pcm_buffer.get_mut_channel_data_view::<f32>(channel as usize);
                        let data_buffer: &[f32];
                        unsafe {
                            let raw_buffer = std::slice::from_raw_parts_mut(
                                (*resample_audio_frame.as_ptr()).data[channel as usize] as *mut f32,
                                (*resample_audio_frame.as_ptr()).linesize[0] as usize,
                            );
                            // let raw_buffer = resample_audio_frame.data(channel as usize);
                            data_buffer = std::slice::from_raw_parts::<f32>(
                                raw_buffer.as_ptr() as *const f32,
                                raw_buffer.len() / std::mem::size_of::<f32>(),
                            );
                        };
                        if data_buffer.len() == buffer.len() {
                            buffer.copy_from_slice(data_buffer);
                        }
                    }
                    let audio_frame = AudioFrame {
                        pcm_buffer,
                        time_range_rational: TimeRangeRational {
                            start: ffmpeg_next::Rational(rescale_start_pts as i32, sample_rate),
                            end: ffmpeg_next::Rational(rescale_end_pts as i32, sample_rate),
                        },
                    };
                    audio_frames.push(audio_frame);
                }
                return Some(audio_frames);
            }
            None => None,
        }
    }

    pub fn get_current_seek_time(&self) -> f32 {
        self.current_seek_time
    }
}

impl Drop for AudioFrameExtractor {
    fn drop(&mut self) {
        if let Some(clean) = self.clean.as_mut() {
            (clean.clean)();
        }
    }
}
