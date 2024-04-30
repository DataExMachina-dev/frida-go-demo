use std::{arch::asm, thread};

use config::Config;
use frida_gum::Gum;
use lazy_static::lazy_static;
use std::ffi::CString;

lazy_static! {
    static ref GUM: Gum = unsafe { Gum::obtain() };
}

#[no_mangle]
extern "C" fn entrypoint(data: *const u8, stay_resident: *mut u32) {
    unsafe {
        *stay_resident = 1;
    }
    let input = unsafe { CString::from_raw(data as *mut i8) };

    thread::spawn(move || {
        let _gum = &*GUM;
        let config: Config = match serde_json::from_slice(input.as_bytes()) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("failed to parse config: {:?}", err);
                return;
            }
        };
        run(config)
    });
}

fn run(config: Config) {
    eprintln!("running");
    let target_function = &config.target_function;
    let func = frida_gum::DebugSymbol::find_function(target_function.as_str());
    let func = match func {
        Some(func) => func,
        None => {
            eprintln!("failed to find func {}!", target_function);
            return;
        }
    };
    let mut interceptor = frida_gum::interceptor::Interceptor::obtain(&GUM);
    match config.action {
        config::Action::MeasureStack => {
            let mut listener = MeasureStackListener::default();
            interceptor.attach_instruction(func, &mut listener);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                eprintln!(
                    "total_size: {}",
                    listener
                        .total_size
                        .load(std::sync::atomic::Ordering::Relaxed),
                );
            }
        }
        config::Action::DoMoreStuff => {
            let mut listener = DoesMoreStuffListener {
                last_printed: std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap(),
            };
            interceptor.attach_instruction(func, &mut listener);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}

#[derive(Default)]
struct MeasureStackListener {
    total_size: std::sync::atomic::AtomicU64,
}

impl frida_gum::interceptor::ProbeListener for MeasureStackListener {
    fn on_hit(&mut self, context: frida_gum::interceptor::InvocationContext) {
        let rsp = read_rsp();
        let total_size = context.cpu_context().rsp() as i64 - rsp as i64;
        self.total_size
            .store(total_size as u64, std::sync::atomic::Ordering::Relaxed);
    }
}

fn read_rsp() -> u64 {
    let rsp: u64;
    unsafe { asm!("mov {}, rsp", out(reg) rsp) };
    rsp
}

struct DoesMoreStuffListener {
    last_printed: std::time::Instant,
}

impl frida_gum::interceptor::ProbeListener for DoesMoreStuffListener {
    fn on_hit(&mut self, context: frida_gum::interceptor::InvocationContext) {
        let rsp = read_rsp();
        let total_size = context.cpu_context().rsp() as i64 - rsp as i64;
        if self.last_printed.elapsed() > std::time::Duration::from_secs(1) {
            eprintln!("total_size: {}", total_size);
            self.last_printed = std::time::Instant::now();
        }
    }
}
