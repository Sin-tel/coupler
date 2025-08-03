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
use log::error;

pub const MAX_BUF_SIZE: usize = 64;

struct Track {
    gain: f32,
}

impl Track {
    fn new() -> Self {
        Track { gain: 1.0 }
    }

    fn process(&mut self, x: f32) -> f32 {
        return x * self.gain;
    }

    fn set_params(&mut self, params: &Params) {
        self.gain = from_db(params.gain);
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

        let mut tracks = Vec::new();

        for _ in 0..n_channels {
            tracks.push(Track::new());
        }

        PluginEngine { params, tracks }
    }

    fn handle_event(&mut self, event: &Event) {
        if let Data::ParamChange { id, value } = event.data {
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

            for track in &mut self.tracks {
                track.set_params(&self.params);
            }

            for mut block in buffer.into_blocks().chunks(MAX_BUF_SIZE) {
                for sample in block.samples() {
                    for (x, track) in sample.into_iter().zip(self.tracks.iter_mut()) {
                        *x = track.process(*x);
                    }
                }

                // for channel in block {
                //     // error!("{}", channel.len());
                //     // todo!()
                //     // for x in channel {}
                //     // panic!("Hello!");
                // }

                panic!("Test");
            }
        }
    }
}
