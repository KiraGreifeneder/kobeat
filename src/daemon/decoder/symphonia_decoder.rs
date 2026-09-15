use anyhow::{Context, Result};
use std::mem::size_of;
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::FormatReader;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTag};
use symphonia::core::units::{Time, Timestamp};

pub struct SymphoniaDecoder {
    format: Box<dyn FormatReader>,
    track_id: u32,                  // used to filter packets.
    decoder: Box<dyn AudioDecoder>, // the decoder for the track.
    sample_rate: u32,
    channels: u16,
    duration_secs: Option<f64>,
    title: Option<String>,
    pub done: bool,
}

impl SymphoniaDecoder {
    pub fn open(path: &str) -> Result<Self> {
        let src = std::fs::File::open(path).context("failed to open media")?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let mut hint = Hint::new();
        if let Some(ext) = path.split('.').last() {
            hint.with_extension(ext);
        }

        let format = symphonia::default::get_probe()
            .probe(&hint, mss, fmt_opts, meta_opts)
            .context("unsupported format")?;

        // Find the first audio track with a known (decodeable) codec.
        let track = (format)
            .default_track(TrackType::Audio)
            .context("no audio track")?;
        let audio_params = track
            .codec_params
            .as_ref()
            .context("codec parameters missing")?
            .audio()
            .context("failed to get audio params")?;
        let sample_rate = audio_params.sample_rate.context("sample rate missing")?;
        let channels = audio_params
            .channels
            .as_ref()
            .map(|channels| channels.count())
            .context("channel layout missing")?
            .try_into()
            .context("channel count exceeds u16")?;

        let dec_opts: AudioDecoderOptions = Default::default();

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &dec_opts)
            .context("unsupported codec")?;

        // Read off the track before `format` gets borrowed again below.
        let track_id = track.id;
        let duration_secs = track.duration.and_then(|duration| {
            let time_base = track.time_base?;
            let time = time_base.calc_time(Timestamp::new(duration.get() as i64))?;
            Some(time.as_secs_f64())
        });

        let mut decoder = SymphoniaDecoder {
            track_id,
            format,
            decoder,
            sample_rate,
            channels,
            duration_secs,
            title: None,
            done: false,
        };
        decoder.title = decoder.read_title();

        Ok(decoder)
    }

    fn read_title(&mut self) -> Option<String> {
        let metadata = self.format.metadata();
        let revision = metadata.current()?;

        revision.media.tags.iter().find_map(|tag| match &tag.std {
            Some(StandardTag::TrackTitle(title)) => Some(title.to_string()),
            _ => None,
        })
    }

    /// Decodes the next packet into `out`. To be called in a loop.
    /// `out` always reflects only this call: it's empty when a packet is
    /// skipped or the track is over.
    pub fn read_pcm_s16le(&mut self, out: &mut Vec<u8>) -> Result<usize> {
        out.clear();

        // Get the next packet from the media format.
        let packet = match self.format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => {
                self.done = true;
                return Ok(0); // end of data
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err(anyhow::Error::new(SymphoniaError::ResetRequired).context(
                    "A reset is required. Please create a new decoder. Did the track change?",
                ));
            }
            Err(err) => {
                return Err(anyhow::Error::new(err).context("failed to get next packet"));
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !self.format.metadata().is_latest() {
            self.format.metadata().pop();
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id != self.track_id {
            return Ok(0);
        }

        // Decode the packet into audio samples.
        match self.decoder.decode(&packet) {
            Ok(audio_buf) => {
                out.resize(audio_buf.samples_interleaved() * size_of::<i16>(), 0);
                audio_buf.copy_bytes_interleaved_as::<i16, _>(out.as_mut_slice());

                Ok(out.len())
            }
            Err(SymphoniaError::IoError(err)) => {
                return Err(anyhow::Error::from(SymphoniaError::IoError(err))
                    .context("Something went wrong while reading/writing data."));
            }
            Err(SymphoniaError::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                eprintln!("Packet failed to decode due to invalid data. Skipping.");
                Ok(0)
            }
            Err(err) => {
                // An unrecoverable error occurred, halt decoding.
                Err(anyhow::Error::new(err).context("failed to decode packet"))
            }
        }
    }

    pub fn seek(&mut self, position_secs: f64) -> Result<()> {
        let time = Time::from_millis_u64((position_secs.max(0.0) * 1000.0) as u64);

        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .context("failed to seek")?;

        self.decoder.reset();
        self.done = false;

        Ok(())
    }

    pub fn duration_secs(&self) -> Option<f64> {
        self.duration_secs
    }

    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }
}
