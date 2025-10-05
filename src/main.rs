use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SupportedBufferSize, SupportedStreamConfig, SupportedStreamConfigRange};
fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let devices = host.devices()?;
    let default_out = host.default_output_device().and_then(|d| d.name().ok());
    let default_in = host.default_input_device().and_then(|d| d.name().ok());

    let default_out_cfg = host
        .default_output_device()
        .and_then(|d| d.default_output_config().ok());
    let default_in_cfg = host
        .default_input_device()
        .and_then(|d| d.default_input_config().ok());

    println!("Default output device: {:?}", default_out.as_ref());
    println!("Default input device: {:?}", default_in.as_ref());
    println!("Default output config: {:?}", default_out_cfg.as_ref());
    println!("Default input config: {:?}", default_in_cfg.as_ref());

    println!("Available devices:");

    for (i, device) in devices.enumerate() {
        let name = device.name()?;

        let is_input = device.supports_input();
        let is_output = device.supports_output();
        let kind = match (is_input, is_output) {
            (true, true) => "input/output",
            (true, false) => "input",
            (false, true) => "output",
            (false, false) => "unknown",
        };

        let def_mark = if Some(name.clone()) == default_out {
            " [default output]"
        } else if Some(name.clone()) == default_in {
            " [default input]"
        } else {
            ""
        };

        println!("\n{i}: {name} ({kind}){def_mark}");

        match device.supported_output_configs() {
            Ok(cfgs) => {
                for cfg in cfgs {
                    let mark = default_out_cfg
                        .as_ref()
                        .filter(|def| match_config(&cfg, def))
                        .map(|_| " [default]")
                        .unwrap_or("");
                    println!("[output] {}{mark}", fmt_cfg(&cfg));
                }
            }
            Err(e) => println!("[output] (no configs / error: {e})"),
        }

        match device.supported_input_configs() {
            Ok(cfgs) => {
                for cfg in cfgs {
                    let mark = default_in_cfg
                        .as_ref()
                        .filter(|def| match_config(&cfg, def))
                        .map(|_| " [default]")
                        .unwrap_or("");
                    println!("[input] {}{mark}", fmt_cfg(&cfg));
                }
            }
            Err(e) => println!("[input] (no configs / error: {e})"),
        }
    }
    Ok(())
}

fn fmt_cfg(cfg: &SupportedStreamConfigRange) -> String {
    let ch = cfg.channels();
    let sf = cfg.sample_format();
    let min = cfg.min_sample_rate().0;
    let max = cfg.max_sample_rate().0;
    let bs = match cfg.buffer_size() {
        SupportedBufferSize::Range { min, max } => format!("{}..={}", min, max),
        other => format!("{:?}", other),
    };
    format!(
        "channels: {ch}, sample_format: {sf:?}, min_rate: {min}, max_rate: {max}, buffer_size: {bs}"
    )
}

fn match_config(range: &SupportedStreamConfigRange, def: &SupportedStreamConfig) -> bool {
    range.channels() == def.channels()
        && range.sample_format() == def.sample_format()
        && def.sample_rate().0 >= range.min_sample_rate().0
        && def.sample_rate().0 <= range.max_sample_rate().0
}
