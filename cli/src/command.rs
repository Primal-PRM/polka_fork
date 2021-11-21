// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use crate::cli::{Cli, Subcommand};
use futures::future::TryFutureExt;
use log::info;
use sc_cli::{Role, RuntimeVersion, SubstrateCli};
use service::{self, IdentifyVariant};
use sp_core::crypto::Ss58AddressFormatRegistry;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	PolkadotService(#[from] service::Error),

	#[error(transparent)]
	SubstrateCli(#[from] sc_cli::Error),

	#[error(transparent)]
	SubstrateService(#[from] sc_service::Error),

	#[error("Other: {0}")]
	Other(String),
}

impl std::convert::From<String> for Error {
	fn from(s: String) -> Self {
		Self::Other(s)
	}
}

type Result<T> = std::result::Result<T, Error>;

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Parity Polkadot".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/polkadot/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn executable_name() -> String {
		"polkadot".into()
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["polkadot", "kusama", "westend", "rococo"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("polkadot")
		} else {
			id
		};
		Ok(match id {
			"kusama" => Box::new(service::chain_spec::kusama_config()?),
			#[cfg(feature = "kusama-native")]
			"kusama-dev" => Box::new(service::chain_spec::kusama_development_config()?),
			#[cfg(feature = "kusama-native")]
			"kusama-local" => Box::new(service::chain_spec::kusama_local_testnet_config()?),
			#[cfg(feature = "kusama-native")]
			"kusama-staging" => Box::new(service::chain_spec::kusama_staging_testnet_config()?),
			#[cfg(not(feature = "kusama-native"))]
			name if name.starts_with("kusama-") && !name.ends_with(".json") =>
				Err(format!("`{}` only supported with `kusama-native` feature enabled.", name))?,
			"polkadot" => Box::new(service::chain_spec::polkadot_config()?),
			#[cfg(feature = "polkadot-native")]
			"polkadot-dev" | "dev" => Box::new(service::chain_spec::polkadot_development_config()?),
			#[cfg(feature = "polkadot-native")]
			"polkadot-local" => Box::new(service::chain_spec::polkadot_local_testnet_config()?),
			#[cfg(feature = "polkadot-native")]
			"polkadot-staging" => Box::new(service::chain_spec::polkadot_staging_testnet_config()?),
			"rococo" => Box::new(service::chain_spec::rococo_config()?),
			#[cfg(feature = "rococo-native")]
			"rococo-dev" => Box::new(service::chain_spec::rococo_development_config()?),
			#[cfg(feature = "rococo-native")]
			"rococo-local" => Box::new(service::chain_spec::rococo_local_testnet_config()?),
			#[cfg(feature = "rococo-native")]
			"rococo-staging" => Box::new(service::chain_spec::rococo_staging_testnet_config()?),
			#[cfg(not(feature = "rococo-native"))]
			name if name.starts_with("rococo-") && !name.ends_with(".json") =>
				Err(format!("`{}` only supported with `rococo-native` feature enabled.", name))?,
			"westend" => Box::new(service::chain_spec::westend_config()?),
			#[cfg(feature = "westend-native")]
			"westend-dev" => Box::new(service::chain_spec::westend_development_config()?),
			#[cfg(feature = "westend-native")]
			"westend-local" => Box::new(service::chain_spec::westend_local_testnet_config()?),
			#[cfg(feature = "westend-native")]
			"westend-staging" => Box::new(service::chain_spec::westend_staging_testnet_config()?),
			#[cfg(not(feature = "westend-native"))]
			name if name.starts_with("westend-") && !name.ends_with(".json") =>
				Err(format!("`{}` only supported with `westend-native` feature enabled.", name))?,
			"wococo" => Box::new(service::chain_spec::wococo_config()?),
			#[cfg(feature = "rococo-native")]
			"wococo-dev" => Box::new(service::chain_spec::wococo_development_config()?),
			#[cfg(feature = "rococo-native")]
			"wococo-local" => Box::new(service::chain_spec::wococo_local_testnet_config()?),
			#[cfg(not(feature = "rococo-native"))]
			name if name.starts_with("wococo-") =>
				Err(format!("`{}` only supported with `rococo-native` feature enabled.", name))?,
			path => {
				let path = std::path::PathBuf::from(path);

				let chain_spec = Box::new(service::PolkadotChainSpec::from_json_file(path.clone())?)
					as Box<dyn service::ChainSpec>;

				// When `force_*` is given or the file name starts with the name of one of the known chains,
				// we use the chain spec for the specific chain.
				if self.run.force_rococo || chain_spec.is_rococo() || chain_spec.is_wococo() {
					Box::new(service::RococoChainSpec::from_json_file(path)?)
				} else if self.run.force_kusama || chain_spec.is_kusama() {
					Box::new(service::KusamaChainSpec::from_json_file(path)?)
				} else if self.run.force_westend || chain_spec.is_westend() {
					Box::new(service::WestendChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			},
		})
	}

	fn native_runtime_version(spec: &Box<dyn service::ChainSpec>) -> &'static RuntimeVersion {
		#[cfg(feature = "kusama-native")]
		if spec.is_kusama() {
			return &service::kusama_runtime::VERSION
		}

		#[cfg(feature = "westend-native")]
		if spec.is_westend() {
			return &service::westend_runtime::VERSION
		}

		#[cfg(feature = "rococo-native")]
		if spec.is_rococo() || spec.is_wococo() {
			return &service::rococo_runtime::VERSION
		}

		#[cfg(not(all(
			feature = "rococo-native",
			feature = "westend-native",
			feature = "kusama-native"
		)))]
		let _ = spec;

		#[cfg(feature = "polkadot-native")]
		{
			return &service::polkadot_runtime::VERSION
		}

		#[cfg(not(feature = "polkadot-native"))]
		panic!("No runtime feature (polkadot, kusama, westend, rococo) is enabled")
	}
}

fn set_default_ss58_version(spec: &Box<dyn service::ChainSpec>) {
	let ss58_version = if spec.is_kusama() {
		Ss58AddressFormatRegistry::KusamaAccount
	} else if spec.is_westend() {
		Ss58AddressFormatRegistry::SubstrateAccount
	} else {
		Ss58AddressFormatRegistry::PolkadotAccount
	}
	.into();

	sp_core::crypto::set_default_ss58_version(ss58_version);
}

const DEV_ONLY_ERROR_PATTERN: &'static str =
	"can only use subcommand with --chain [polkadot-dev, kusama-dev, westend-dev, rococo-dev, wococo-dev], got ";

fn ensure_dev(spec: &Box<dyn service::ChainSpec>) -> std::result::Result<(), String> {
	if spec.is_dev() {
		Ok(())
	} else {
		Err(format!("{}{}", DEV_ONLY_ERROR_PATTERN, spec.id()))
	}
}

/// Runs a performance check via compiling sample wasm code with a timeout.
/// Only available in release build since the check would take too much time otherwise.
/// Returns `Ok` if the check has been passed previously.
#[cfg(not(debug_assertions))]
fn host_perf_check() -> Result<()> {
	use polkadot_node_core_pvf::sp_maybe_compressed_blob;
	use std::{fs::OpenOptions, path::Path, time::Duration};

	const PERF_CHECK_TIME_LIMIT: Duration = Duration::from_secs(20);
	const CODE_SIZE_LIMIT: usize = 1024usize.pow(3);
	const WASM_CODE: &[u8] = include_bytes!(
		"../../target/release/wbuild/kusama-runtime/kusama_runtime.compact.compressed.wasm"
	);
	const CHECK_PASSED_FILE_NAME: &str = ".perf_check_passed";

	// We will try to save a dummy file to the same path as the polkadot binary
	// to make it independent from the current directory.
	let check_passed_path = std::env::current_exe()
		.map(|mut path| {
			path.pop();
			path
		})
		.unwrap_or_default()
		.join(CHECK_PASSED_FILE_NAME);

	// To avoid running the check on every launch we create a dummy dot-file on success.
	if Path::new(&check_passed_path).exists() {
		info!("Performance check skipped: already passed");
		return Ok(())
	}

	info!("Running the performance check...");
	let start = std::time::Instant::now();

	// Recreate the pipeline from the pvf prepare worker.
	let code = sp_maybe_compressed_blob::decompress(WASM_CODE, CODE_SIZE_LIMIT).map_err(|err| {
		Error::Other(format!("Failed to decompress test wasm code: {}", err.to_string()))
	})?;
	let blob = polkadot_node_core_pvf::prevalidate(code.as_ref()).map_err(|err| {
		Error::Other(format!(
			"Failed to create runtime blob from the decompressed code: {}",
			err.to_string()
		))
	})?;
	let _ = polkadot_node_core_pvf::prepare(blob).map_err(|err| {
		Error::Other(format!("Failed to precompile test wasm code: {}", err.to_string()))
	})?;

	let elapsed = start.elapsed();
	if elapsed <= PERF_CHECK_TIME_LIMIT {
		info!("Performance check passed, elapsed: {:?}", start.elapsed());
		// `touch` a dummy file.
		let _ = OpenOptions::new().create(true).write(true).open(Path::new(&check_passed_path));
		Ok(())
	} else {
		Err(Error::Other(format!(
			"Performance check not passed: exceeded the {:?} time limit, elapsed: {:?}",
			PERF_CHECK_TIME_LIMIT, elapsed
		)))
	}
}

/// Launch a node, accepting arguments just like a regular node,
/// accepts an alternative overseer generator, to adjust behavior
/// for integration tests as needed.
#[cfg(feature = "malus")]
pub fn run_node(cli: Cli, overseer_gen: impl service::OverseerGen) -> Result<()> {
	run_node_inner(cli, overseer_gen)
}

fn run_node_inner(cli: Cli, overseer_gen: impl service::OverseerGen) -> Result<()> {
	let runner = cli.create_runner(&cli.run.base).map_err(Error::from)?;
	let chain_spec = &runner.config().chain_spec;

	set_default_ss58_version(chain_spec);

	let grandpa_pause = if cli.run.grandpa_pause.is_empty() {
		None
	} else {
		Some((cli.run.grandpa_pause[0], cli.run.grandpa_pause[1]))
	};

	if chain_spec.is_kusama() {
		info!("----------------------------");
		info!("This chain is not in any way");
		info!("      endorsed by the       ");
		info!("     KUSAMA FOUNDATION      ");
		info!("----------------------------");
	}

	let jaeger_agent = cli.run.jaeger_agent;

	runner.run_node_until_exit(move |config| async move {
		let role = config.role.clone();

		match role {
			Role::Light => Err(Error::Other("Light client not enabled".into())),
			_ => service::build_full(
				config,
				service::IsCollator::No,
				grandpa_pause,
				cli.run.no_beefy,
				jaeger_agent,
				None,
				overseer_gen,
			)
			.map(|full| full.task_manager)
			.map_err(Into::into),
		}
	})
}

/// Parses polkadot specific CLI arguments and run the service.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => run_node_inner(cli, service::RealOverseerGen),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd).map_err(Error::SubstrateCli)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) =
					service::new_chain_ops(&mut config, None).map_err(Error::PolkadotService)?;
				Ok((cmd.run(client, config.database).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, config.chain_spec).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.database))?)
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, backend).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::PvfPrepareWorker(cmd)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_colors(false);
			let _ = builder.init();

			#[cfg(target_os = "android")]
			{
				return Err(sc_cli::Error::Input(
					"PVF preparation workers are not supported under this platform".into(),
				)
				.into())
			}

			#[cfg(not(target_os = "android"))]
			{
				polkadot_node_core_pvf::prepare_worker_entrypoint(&cmd.socket_path);
				Ok(())
			}
		},
		Some(Subcommand::PvfExecuteWorker(cmd)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_colors(false);
			let _ = builder.init();

			#[cfg(target_os = "android")]
			{
				return Err(sc_cli::Error::Input(
					"PVF execution workers are not supported under this platform".into(),
				)
				.into())
			}

			#[cfg(not(target_os = "android"))]
			{
				polkadot_node_core_pvf::execute_worker_entrypoint(&cmd.socket_path);
				Ok(())
			}
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			ensure_dev(chain_spec).map_err(Error::Other)?;

			#[cfg(feature = "kusama-native")]
			if chain_spec.is_kusama() {
				return Ok(runner.sync_run(|config| {
					cmd.run::<service::kusama_runtime::Block, service::KusamaExecutorDispatch>(
						config,
					)
					.map_err(|e| Error::SubstrateCli(e))
				})?)
			}

			#[cfg(feature = "westend-native")]
			if chain_spec.is_westend() {
				return Ok(runner.sync_run(|config| {
					cmd.run::<service::westend_runtime::Block, service::WestendExecutorDispatch>(
						config,
					)
					.map_err(|e| Error::SubstrateCli(e))
				})?)
			}

			// else we assume it is polkadot.
			#[cfg(feature = "polkadot-native")]
			{
				return Ok(runner.sync_run(|config| {
					cmd.run::<service::polkadot_runtime::Block, service::PolkadotExecutorDispatch>(
						config,
					)
					.map_err(|e| Error::SubstrateCli(e))
				})?)
			}
			#[cfg(not(feature = "polkadot-native"))]
			panic!("No runtime feature (polkadot, kusama, westend, rococo) is enabled")
		},
		#[cfg(not(debug_assertions))]
		Some(Subcommand::HostPerfCheck) => {
			let mut builder = sc_cli::LoggerBuilder::new("").with_colors(true);
			let _ = builder.init();

			host_perf_check()
		},
		Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			use sc_service::TaskManager;
			let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
			let task_manager = TaskManager::new(runner.config().tokio_handle.clone(), *registry)
				.map_err(|e| Error::SubstrateService(sc_service::Error::Prometheus(e)))?;

			ensure_dev(chain_spec).map_err(Error::Other)?;

			#[cfg(feature = "kusama-native")]
			if chain_spec.is_kusama() {
				return runner.async_run(|config| {
					Ok((
						cmd.run::<service::kusama_runtime::Block, service::KusamaExecutorDispatch>(
							config,
						)
						.map_err(Error::SubstrateCli),
						task_manager,
					))
				})
			}

			#[cfg(feature = "westend-native")]
			if chain_spec.is_westend() {
				return runner.async_run(|config| {
					Ok((
						cmd.run::<service::westend_runtime::Block, service::WestendExecutorDispatch>(
							config,
						)
						.map_err(Error::SubstrateCli),
						task_manager,
					))
				})
			}
			// else we assume it is polkadot.
			#[cfg(feature = "polkadot-native")]
			{
				return runner.async_run(|config| {
					Ok((
						cmd.run::<service::polkadot_runtime::Block, service::PolkadotExecutorDispatch>(
							config,
						)
						.map_err(Error::SubstrateCli),
						task_manager,
					))
				})
			}
			#[cfg(not(feature = "polkadot-native"))]
			panic!("No runtime feature (polkadot, kusama, westend, rococo) is enabled")
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err(Error::Other(
			"TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
				.into(),
		)
		.into()),
	}?;
	Ok(())
}
