use std::{
  borrow::{Borrow, Cow},
  fs::File,
  path::PathBuf,
  process::exit,
};

use anyhow::Result;
use memmap2::Mmap;
use once_cell::sync::OnceCell;

use crate::log::DefaultLevel;
use actix_web::{middleware, web, App, HttpServer};
use anyhow::Context;
use clap::{Parser, ValueHint};
use clap_verbosity_flag::Verbosity;

mod ipdb;
mod log;
mod route;

/// A simple IPDB server with low memory footprint
/// and high performance.
#[derive(Parser, Clone)]
#[clap(name = "ipdb-server", bin_name = "ipdb-server", version, about)]
struct Cli {
  /// The address to listen on
  #[clap(short, long, default_value_t = Cow::from("localhost"), env = "IPDB_SERVER_ADDR")]
  addr: Cow<'static, str>,
  /// The port to listen on
  #[clap(short, long, default_value_t = 26583, env = "IPDB_SERVER_PORT")]
  port: u16,
  /// IPDB IPv4 path
  #[clap(short = '4', long, env = "IPDB_SERVER_V4")]
  #[clap(value_hint = ValueHint::FilePath)]
  v4_path: Option<PathBuf>,
  /// IPDB IPv6 path
  #[clap(short = '6', long, env = "IPDB_SERVER_V4")]
  #[clap(value_hint = ValueHint::FilePath)]
  v6_path: Option<PathBuf>,
  #[clap(short = 't', long, env = "IPDB_SERVER_TOKEN")]
  token: Option<String>,
  /// log config, default None, use default config, control level by following options
  #[clap(short, long, env = "IPDB_SERVER_LOG_CONFIG")]
  #[clap(value_hint = ValueHint::FilePath)]
  log_config: Option<PathBuf>,
  #[clap(flatten)]
  verbose: Verbosity<DefaultLevel>,
}

static TOKEN: OnceCell<Option<String>> = OnceCell::new();
static IPV4_DB: OnceCell<ipdb::Reader> = OnceCell::new();
static IPV6_DB: OnceCell<ipdb::Reader> = OnceCell::new();

fn load_db(cell: &OnceCell<ipdb::Reader>, path: Option<PathBuf>) -> anyhow::Result<bool> {
  let Some(path) =path else {
    return Ok(false)
  };
  cell.get_or_try_init(|| {
    let file = File::open(&path)
      .with_context(|| format!("Failed to open file `{}`", path.as_path().to_string_lossy()))?;
    let mmap = unsafe { Mmap::map(&file) }.with_context(|| {
      format!(
        "Failed to map file `{}` to memory",
        path.as_path().to_string_lossy()
      )
    })?;
    let boxed = Box::new(mmap);
    let leak = Box::leak(boxed);
    ipdb::Reader::new(leak)
      .with_context(|| format!("Failed to load ipdb {}", path.as_path().to_string_lossy()))
  })?;
  Ok(true)
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  log::setup(cli.verbose.log_level_filter(), cli.log_config).context("Failed to setup logger")?;

  if !load_db(&IPV4_DB, cli.v4_path)? && !load_db(&IPV6_DB, cli.v6_path)? {
    eprintln!("IPv4 and IPv6 IPDB are both not provided, please pass parameter by `-4 <path>` or `-6 <path>`");
    exit(1)
  }
  TOKEN.get_or_init(|| cli.token);

  HttpServer::new(|| {
    App::new()
      .wrap(middleware::Compress::default())
      .service(route::root)
      .service(route::ip)
      .default_service(web::to(route::default))
  })
  .bind((cli.addr.borrow(), cli.port))
  .context("Failed to bind socket")?
  .run()
  .await
  .context("Failed to start server")
}
