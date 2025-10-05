use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig, SupportedStreamConfigRange,
};
fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let devices = host.devices()?;
    let default_out_device = host.default_output_device();
    let default_in_device = host.default_input_device();
    let default_out_device_name = default_out_device.as_ref().and_then(|d| d.name().ok());
    let default_in_device_name = default_in_device.as_ref().and_then(|d| d.name().ok());

    let default_out_cfg = host
        .default_output_device()
        .and_then(|d| d.default_output_config().ok());
    let default_in_cfg = host
        .default_input_device()
        .and_then(|d| d.default_input_config().ok());

    println!(
        "Default output device: {:?}",
        default_out_device_name.as_ref()
    );
    println!(
        "Default input device: {:?}",
        default_in_device_name.as_ref()
    );
    println!("Default output config: {:?}", default_out_cfg.as_ref());
    println!("Default input config: {:?}", default_in_cfg.as_ref());

    println!("Available devices:");

    for (i, device) in devices.enumerate() {
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());

        let is_input = device.supports_input();
        let is_output = device.supports_output();
        let kind = match (is_input, is_output) {
            (true, true) => "input/output",
            (true, false) => "input",
            (false, true) => "output",
            (false, false) => "unknown",
        };

        let def_mark = if Some(name.clone()) == default_out_device_name {
            " [default output]"
        } else if Some(name.clone()) == default_in_device_name {
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

    const REQUESTED_SR: u32 = 48_000;
    const REQUESTED_CH: u16 = 2;
    const REQUESTED_BUFFER_FRAMES: u32 = 256;
    println!(
        "\nRequested: SR={} Hz, Channels={}, Buffer={} frames",
        REQUESTED_SR, REQUESTED_CH, REQUESTED_BUFFER_FRAMES
    );
    let config = StreamConfig {
        channels: REQUESTED_CH,
        sample_rate: SampleRate(REQUESTED_SR),
        buffer_size: BufferSize::Fixed(REQUESTED_BUFFER_FRAMES),
    };

    let sample_format = default_out_device
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No default output device"))?
        .default_output_config()?
        .sample_format();

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(default_out_device.as_ref().unwrap(), &config)?,
        other => anyhow::bail!("Unsupported sample format: {:?}", other),
    };
    drop(stream);

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

fn build_stream<T>(device: &Device, config: &StreamConfig) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::SizedSample + num_traits::Zero,
{
    let err_fn = |e| eprintln!("[stream error] {e}");
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            // 無音：ゼロ埋め（ログは出さない：RT負荷になるため）
            for s in data.iter_mut() {
                *s = T::zero();
            }
        },
        err_fn,
        None, // latency hint は未指定（今日は扱わない）
    )?;
    Ok(stream)
}
