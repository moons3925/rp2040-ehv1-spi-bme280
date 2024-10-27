use core::panic::PanicInfo;

use crate::print;
use crate::println;

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    println!("{}", panic_info);
    loop {}
}
