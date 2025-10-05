use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig, SupportedStreamConfigRange,
};
use std::fmt::Debug;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about = "Minimal CPAL output test")]
struct Args {
    /// Sample rate (Hz)
    #[arg(long, default_value_t = 48000)]
    sr: u32,

    /// Channel count
    #[arg(long, default_value_t = 2)]
    ch: u16,

    /// Buffer size (frames)
    #[arg(long, default_value_t = 256)]
    buffer: u32,

    /// Log every N callbacks (0=disable, 1=every time). Day1は非RTセーフなprintlnで観察する
    #[arg(long, default_value_t = 10)]
    log_every: u64,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

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

    println!(
        "\nRequested: SR={} Hz, Channels={}, Buffer={} frames",
        args.sr, args.ch, args.buffer
    );
    let config = StreamConfig {
        channels: args.ch,
        sample_rate: SampleRate(args.sr),
        buffer_size: BufferSize::Fixed(args.buffer),
    };

    let sample_format = default_out_device
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No default output device"))?
        .default_output_config()?
        .sample_format();

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(
            default_out_device.as_ref().unwrap(),
            &config,
            args.log_every,
        )?,
        other => anyhow::bail!("Unsupported sample format: {:?}", other),
    };

    stream.play()?;
    thread::sleep(Duration::from_secs(3));
    println!("Stopped after 3s.");

    println!(
        "Actual   : SR={} Hz, Ch={} (Buffer/Latency=N/A; backend-dependent)",
        config.sample_rate.0, config.channels,
    );
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

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    log_every: u64,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::SizedSample + num_traits::Zero,
{
    let err_fn = |e| eprintln!("[stream error] {e}");
    let mut last_time = std::time::Instant::now();
    let mut last_len: usize = 0;
    let mut n: u64 = 0;

    let channels = config.channels;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _info| {
            if log_every > 0 {
                n += 1;
                let now = std::time::Instant::now();
                let dt = now.duration_since(last_time).as_secs_f64();
                last_time = now;

                let len = data.len();
                if len != last_len && last_len != 0 {
                    println!(
                        "⚠️ buffer size changed: {} -> {} (frames per callback)",
                        last_len, len
                    );
                }
                last_len = len;

                if n % log_every == 0 {
                    let ch = channels as usize;
                    let frames = if ch > 0 { len / ch } else { len };
                    println!(
                        "[cb #{:>6}] frames/cb: {:>5} | samples: {:>5} | Δt={:.6}s",
                        n, frames, len, dt
                    );
                }
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}
