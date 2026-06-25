use anyhow::{Context, Result};
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::FormatReader;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

pub struct SymphoniaDecoder {
    format: Box<dyn FormatReader>,
    track_id: u32,                  // used to filter packets.
    decoder: Box<dyn AudioDecoder>, // the decoder for the track.
    done: bool,
}

impl SymphoniaDecoder {
    pub fn open(&mut self, path: &str) -> Result<()> {
        let src = std::fs::File::open(path).context("failed to open media")?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let mut hint = Hint::new();
        if let Some(ext) = path.split('.').last() {
            hint.with_extension(ext);
        }

        self.format = symphonia::default::get_probe()
            .probe(&hint, mss, fmt_opts, meta_opts)
            .context("unsupported format")?;

        // Find the first audio track with a known (decodeable) codec.
        let track = (self.format)
            .default_track(TrackType::Audio)
            .context("no audio track")?;

        let dec_opts: AudioDecoderOptions = Default::default();

        self.decoder = symphonia::default::get_codecs()
            .make_audio_decoder(
                track
                    .codec_params
                    .as_ref()
                    .context("codec parameters missing")?
                    .audio()
                    .context("failed to get audio params")?,
                &dec_opts,
            )
            .context("unsupported codec")?;

        self.track_id = track.id;

        Ok(())
    }

    // to be called in a loop.
    pub fn read_pcm_i16(&mut self, out: &mut Vec<i16>) -> Result<usize> {
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
                let mut samples: Vec<i16> = Default::default();
                samples.resize(audio_buf.samples_interleaved(), i16::MAX);
                audio_buf.copy_to_slice_interleaved(&mut samples);

                out.clear();
                out.extend_from_slice(&samples);

                Ok(samples.len())
            }
            Err(SymphoniaError::IoError(_)) => {
                Ok(0)
            }
            Err(SymphoniaError::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                Ok(0)
            }
            Err(err) => {
                // An unrecoverable error occurred, halt decoding.
                Err(anyhow::Error::new(err).context("failed to decode packet"))
            }
        }
    }

    pub fn seek(&mut self, _position_secs: f64) -> Result<()> {
        anyhow::bail!("seek is not implemented yet")
    }
}
