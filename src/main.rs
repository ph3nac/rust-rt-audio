use cpal::traits::{DeviceTrait, HostTrait};
fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let devices = host.devices()?;
    let default_out = host.default_output_device().and_then(|d| d.name().ok());
    let default_in = host.default_input_device().and_then(|d| d.name().ok());

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
                    println!("[output] {}", fmt_cfg(&cfg));
                }
            }
            Err(e) => println!("[output] (no configs / error: {e})"),
        }

        match device.supported_input_configs() {
            Ok(cfgs) => {
                for cfg in cfgs {
                    println!("[input] {}", fmt_cfg(&cfg));
                }
            }
            Err(e) => println!("[input] (no configs / error: {e})"),
        }
    }
    Ok(())
}

fn fmt_cfg(cfg: &cpal::SupportedStreamConfigRange) -> String {
    let ch = cfg.channels();
    let sf = cfg.sample_format();
    let min = cfg.min_sample_rate().0;
    let max = cfg.max_sample_rate().0;
    let bs = match cfg.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => format!("{}..={}", min, max),
        other => format!("{:?}", other),
    };
    format!(
        "channels: {ch}, sample_format: {sf:?}, min_rate: {min}, max_rate: {max}, buffer_size: {bs}"
    )
}
