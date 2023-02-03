use core::{
    panic::PanicInfo,
    sync::atomic::{compiler_fence, Ordering::SeqCst},
};

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use cortex_m::interrupt;

    interrupt::disable();

    let w = unsafe { crate::logger::get_logger() };
    writeln!(w, "\n********************************").ok();
    writeln!(w, "{info}").ok();
    writeln!(w, "********************************").ok();

    loop {
        compiler_fence(SeqCst);
    }
}
