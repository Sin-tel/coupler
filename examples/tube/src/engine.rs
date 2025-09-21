use crate::dsp::delayline::DelayLine;
use crate::dsp::onepole::OnePole;
use crate::dsp::resample::*;
use crate::dsp::simper::Filter;
use crate::dsp::smooth::SmoothBuffer;
use crate::dsp::*;
use crate::Params;

use coupler::buffers::iter::BlockIterator;
use coupler::buffers::iter::IntoBlocks;
use coupler::buffers::BufferMut;
use coupler::buffers::Buffers;
use coupler::engine::Config;
use coupler::engine::Engine;
use coupler::events::Data;
use coupler::events::Event;
use coupler::events::Events;
use coupler::params::Params as CouplerParams;
use log::info;

pub const MAX_BUF_SIZE: usize = 64;
pub const RESAMPLE_FACTOR: f32 = 2.0;
pub const RESAMPLE_DELAY: u64 = 16;

fn tube(x: f32) -> f32 {
    let w = x.max(0.0);
    let s = x + 0.13 * w.powi(3) + 0.407 * w.powi(4);
    softclip(s)
}

struct Track {
    gain_in: SmoothBuffer,
    gain_out: SmoothBuffer,
    balance: SmoothBuffer,

    release: f32,
    peak: f32,

    peak_input_filter: Filter,
    peak_filter: Filter,
    highpass_out: Filter,

    pre_filter: OnePole,
    post_filter: OnePole,

    dry_delay: DelayLine,
    upsampler: Upsampler19,
    downsampler: Downsampler51,
}

impl Track {
    fn new(sample_rate: f32) -> Self {
        let internal_sr = sample_rate * RESAMPLE_FACTOR;
        let mut peak_input_filter = Filter::new(internal_sr);
        peak_input_filter.set_highpass(50.0, 0.7);

        let mut peak_filter = Filter::new(internal_sr);
        peak_filter.set_lowpass(5.0, 0.7);

        let mut highpass_out = Filter::new(internal_sr);
        highpass_out.set_highpass(10.0, 0.7);

        let mut pre_filter = OnePole::new(internal_sr);
        pre_filter.set_tilt(180.0, 4.5);

        let mut post_filter = OnePole::new(internal_sr);
        post_filter.set_tilt(180.0, -4.5);

        let release = time_constant(360.0, internal_sr);
        info!("release = {release:?}");

        Track {
            gain_in: SmoothBuffer::new(),
            gain_out: SmoothBuffer::new(),
            balance: SmoothBuffer::new(),

            release,
            peak: 0.0,

            peak_input_filter,
            peak_filter,
            highpass_out,
            pre_filter,
            post_filter,

            dry_delay: DelayLine::new_absolute(sample_rate, 32),
            upsampler: Upsampler19::default(),
            downsampler: Downsampler51::default(),
        }
    }

    fn process_sample(&mut self, i: usize, x: f32) -> f32 {
        let mut s = x * self.gain_in.get(i);

        let peak = self.peak_input_filter.process(s).abs();

        if peak > self.peak {
            self.peak = peak
        } else {
            self.peak = self.peak - (self.peak - peak) * self.release
        }

        let w = self.peak_filter.process(self.peak);

        s = self.pre_filter.process(s);

        let mut out = tube(s + 0.25 - 0.36 * w);

        out = self.post_filter.process(out);
        out = self.highpass_out.process(out);
        out *= self.gain_out.get(i);

        out
    }

    fn process(&mut self, samples: &mut [f32]) {
        let n = samples.len();
        self.gain_in.process_buffer(n);
        self.gain_out.process_buffer(n);
        self.balance.process_buffer(n);

        for (i, sample) in samples.iter_mut().enumerate() {
            let (u1, u2) = self.upsampler.process(*sample);

            let y1 = self.process_sample(i, u1);
            let y2 = self.process_sample(i, u2);

            let y = self.downsampler.process(y1, y2);

            self.dry_delay.push(*sample);
            let dry = self.dry_delay.go_back_int_s(RESAMPLE_DELAY as isize);

            let balance = self.balance.get(i);
            *sample = lerp(dry, y, balance);
        }
    }

    fn set_params(&mut self, params: &Params) {
        self.balance.set(params.balance);
        self.gain_in.set(from_db(params.gain));

        let mut gain_out = 1.0;

        if params.gain > 0.0 {
            gain_out *= from_db(-params.gain * 0.75);
        } else {
            gain_out *= from_db(-params.gain);
        }

        gain_out *= from_db(params.gain_out);

        self.gain_out.set(gain_out);
    }
}

pub struct PluginEngine {
    params: Params,
    tracks: Vec<Track>,
}

impl PluginEngine {
    pub fn new(params: Params, config: Config) -> Self {
        assert!(config.layout.formats.len() == 1);
        let format = &config.layout.formats[0];

        let n_channels = format.channel_count();
        let sample_rate = config.sample_rate as f32;

        info!("n_channels = {n_channels:?}");
        info!("sample_rate = {sample_rate:?}");

        let mut tracks = Vec::new();

        for _ in 0..n_channels {
            tracks.push(Track::new(sample_rate));
        }

        PluginEngine { params, tracks }
    }

    fn handle_event(&mut self, event: &Event) {
        if let Data::ParamChange { id, value } = event.data {
            info!("id = {id:?}, value = {value:?}");

            self.params.set_param(id, value);
        }
    }
}

impl Engine for PluginEngine {
    fn reset(&mut self) {}

    fn flush(&mut self, events: Events) {
        for event in events {
            self.handle_event(event);
        }
    }

    fn process(&mut self, buffers: Buffers, events: Events) {
        let mut buffers: (BufferMut,) = buffers.try_into().unwrap();

        for (buffer, events) in buffers.0.split_at_events(events) {
            for event in events {
                self.handle_event(event);
            }

            if events.len() > 0 {
                for track in &mut self.tracks {
                    track.set_params(&self.params);
                }
            }

            for block in buffer.into_blocks().chunks(MAX_BUF_SIZE) {
                for (buffer, track) in block.into_iter().zip(self.tracks.iter_mut()) {
                    track.process(buffer);
                }
            }
        }
    }
}
