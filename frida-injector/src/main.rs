use clap::Parser as _;
use config::Config;
use frida::Inject;

#[derive(clap::Parser)]
struct Opts {
    #[arg(long)]
    target: String,

    #[arg(long)]
    function: String,

    #[arg(long, value_enum)]
    action: config::Action,

    #[arg(long)]
    lib_to_inject: String,
}

fn main() {
    let opts = Opts::parse();

    let target_pid: u32 = opts.target.parse().expect("failed to parse target pid");

    let frida = unsafe { frida::Frida::obtain() };
    let device_manager = frida::DeviceManager::obtain(&frida);
    let devices = device_manager.enumerate_all_devices();
    let device = devices.first().unwrap();
    device
        .enumerate_processes()
        .into_iter()
        .find(|process| process.get_pid() == target_pid)
        .unwrap_or_else(|| {
            panic!("no process {target_pid} found");
        });

    let config = Config {
        target_function: opts.function,
        action: opts.action,
    };
    let serialized = serde_json::to_vec(&config).unwrap();

    let mut injector = frida::Injector::new();
    let _injection = injector
        .inject_library_file_sync(target_pid, opts.lib_to_inject, "entrypoint", serialized)
        .unwrap_or_else(|err| {
            panic!("Failed to inject library: {err:?}");
        });
}
