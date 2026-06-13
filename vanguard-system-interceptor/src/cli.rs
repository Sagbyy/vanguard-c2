use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(long)]
    pub name: String,

    #[arg(short = 'n', long, default_value_t = 4)]
    pub interceptors: usize,

    #[arg(short, long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub x: f64,

    #[arg(short, long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub y: f64,

    /// Radar detection range in metres (short for in-city point defence).
    #[arg(long, default_value_t = 20_000.0)]
    pub reach: f64,
}
