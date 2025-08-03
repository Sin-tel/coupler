mod engine;
mod logging;

#[allow(dead_code)]
mod dsp;

use engine::PluginEngine;
use logging::init_logging;

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

use coupler::params::Params as CouplerParams;
use coupler::plugin::Plugin as CouplerPlugin;

use coupler::engine::Config;
use coupler::format::clap::*;
use coupler::format::vst3::*;
use coupler::params::ParamId;
use coupler::plugin::PluginInfo;
use coupler::view::ParentWindow;
use coupler::{bus::*, host::*, view::*};

#[derive(CouplerParams, Serialize, Deserialize, Clone)]
pub struct Params {
    #[param(id = 0, name = "Dry/Wet", range = 0.0..1.0, format = "{:.2}")]
    balance: f32,
    #[param(id = 1, name = "Heat", range = -12.0..12.0, format = "{:.1}dB")]
    gain: f32,
    #[param(id = 2, name = "Output gain", range = 0.0..12.0, format = "{:.1}dB")]
    gain_out: f32,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            balance: 1.0,
            gain: 0.0,
            gain_out: 0.0,
        }
    }
}

pub struct Plugin {
    params: Params,
}

impl CouplerPlugin for Plugin {
    type Engine = PluginEngine;
    type View = NoView;

    fn info() -> PluginInfo {
        PluginInfo {
            name: "Sintel's Secret Mojo Sauce Tube".to_string(),
            version: "0.1.0".to_string(),
            vendor: "Sintel".to_string(),
            url: "https://sintel.website".to_string(),
            email: "sintel.inquiries@gmail.com".to_string(),
            buses: vec![BusInfo {
                name: "Main".to_string(),
                dir: BusDir::InOut,
            }],
            layouts: vec![
                Layout {
                    formats: vec![Format::Stereo],
                },
                Layout {
                    formats: vec![Format::Mono],
                },
            ],
            params: Params::params(),
            has_view: false,
        }
    }

    fn new(_host: Host) -> Self {
        init_logging();
        Plugin {
            params: Params::default(),
        }
    }

    fn set_param(&mut self, id: coupler::params::ParamId, value: coupler::params::ParamValue) {
        self.params.set_param(id, value);
    }

    fn get_param(&self, id: ParamId) -> coupler::params::ParamValue {
        self.params.get_param(id)
    }

    fn save(&self, output: &mut impl Write) -> io::Result<()> {
        serde_json::to_writer(output, &self.params)?;

        Ok(())
    }

    fn load(&mut self, input: &mut impl Read) -> io::Result<()> {
        self.params = serde_json::from_reader(input)?;

        Ok(())
    }

    fn engine(&mut self, config: Config) -> Self::Engine {
        PluginEngine::new(self.params.clone(), config)
    }

    fn view(&mut self, _host: ViewHost, _parent: &ParentWindow) -> Self::View {
        NoView
    }
}

impl Vst3Plugin for Plugin {
    fn vst3_info() -> Vst3Info {
        Vst3Info {
            class_id: Uuid(0xDB55AE3A, 0x66F446B6, 0xBF586169, 0x83FD9853),
        }
    }
}

impl ClapPlugin for Plugin {
    fn clap_info() -> ClapInfo {
        ClapInfo {
            id: "rs.sintel.tube".to_string(),
        }
    }
}
