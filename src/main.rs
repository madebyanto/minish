#![no_std]
#![no_main]
// The runtime symbols below must keep the classic C shape (`void *`, and `strlen`'s
// `const char *`) because that is how `core` declares and calls them on this target.
#![allow(suspicious_runtime_symbol_definitions)]

use core::arch::asm;
use core::ffi::{c_int, c_void};
use core::panic::PanicInfo;

const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_FORK: usize = 57;
const SYS_EXECVE: usize = 59;
const SYS_EXIT: usize = 60;
const SYS_WAITPID: usize = 61;
const SYS_GETCWD: usize = 79;
const SYS_CHDIR: usize = 80;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_exit(1);
}

// The freestanding Rinux target (`x86_64-rinux.json`) has no C runtime, so this binary is
// its own entry point: `_start` gets control straight from the kernel.
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let args: [*const u8; 1] = [core::ptr::null()];
    core::hint::black_box(minish_main(1, args.as_ptr()));
    sys_exit(0);
}

#[unsafe(no_mangle)]
extern "C" fn minish_main(_argc: c_int, _argv: *const *const u8) -> c_int {
    let mut input_buf = [0u8; 256];
    let mut cwd_buf = [0u8; 128];

    loop {
        if sys_getcwd(&mut cwd_buf) > 0 {
            print_cstr(cwd_buf.as_ptr());
            print(" > ");
        } else {
            print("minish > ");
        }

        let n = sys_read(0, &mut input_buf);
        if n == 0 {
            print("\n");
            break;
        }
        if n < 0 {
            print("Read error\n");
            continue;
        }

        let mut len = n as usize;
        if len > 0 && input_buf[len - 1] == b'\n' {
            len -= 1;
        }
        if len == 0 {
            continue;
        }
        input_buf[len] = 0;

        let mut argv: [*const u8; 16] = [core::ptr::null(); 16];
        let mut argc = 0;
        let mut in_word = false;

        for i in 0..len {
            if input_buf[i] == b' ' || input_buf[i] == b'\t' {
                input_buf[i] = 0;
                in_word = false;
            } else if !in_word && argc < 15 {
                argv[argc] = unsafe { input_buf.as_ptr().add(i) };
                argc += 1;
                in_word = true;
            }
        }

        if argc == 0 {
            continue;
        }

        if cstr_eq(argv[0], b"exit\0".as_ptr()) {
            break;
        }

        if cstr_eq(argv[0], b"cd\0".as_ptr()) {
            let target = if argc > 1 { argv[1] } else { b"/\0".as_ptr() };
            if sys_chdir(target) < 0 {
                print("cd: failed\n");
            }
            continue;
        }

        execute_external(&argv);
    }

    0
}

fn execute_external(argv: &[*const u8; 16]) {
    let mut path_buf = [0u8; 128];
    let cmd = argv[0];

    let first_byte = unsafe { *cmd };
    let exec_path = if first_byte == b'/' || first_byte == b'.' {
        cmd
    } else {
        let prefix = b"/usr/bin/";
        let mut idx = 0;
        while idx < prefix.len() {
            path_buf[idx] = prefix[idx];
            idx += 1;
        }
        let mut c_idx = 0;
        loop {
            let b = unsafe { *cmd.add(c_idx) };
            if idx < path_buf.len() - 1 {
                path_buf[idx] = b;
                idx += 1;
            }
            if b == 0 {
                break;
            }
            c_idx += 1;
        }
        path_buf.as_ptr()
    };

    let pid = sys_fork();
    if pid == 0 {
        let envp: [*const u8; 1] = [core::ptr::null()];
        sys_execve(exec_path, argv.as_ptr(), envp.as_ptr());
        print("minish: command not found or exec failed\n");
        sys_exit(127);
    } else if pid > 0 {
        let mut status: i32 = 0;
        sys_waitpid(pid, &mut status, 0);
    } else {
        print("minish: fork failed\n");
    }
}

fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_READ => ret,
            in("rdi") fd,
            in("rsi") buf.as_mut_ptr(),
            in("rdx") buf.len(),
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_write(fd: usize, buf: &[u8]) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_WRITE => ret,
            in("rdi") fd,
            in("rsi") buf.as_ptr(),
            in("rdx") buf.len(),
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_fork() -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_FORK => ret,
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_EXECVE => ret,
            in("rdi") path,
            in("rsi") argv,
            in("rdx") envp,
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_exit(code: usize) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_EXIT,
            in("rdi") code,
            options(noreturn)
        );
    }
}

fn sys_waitpid(pid: isize, status: *mut i32, options: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_WAITPID => ret,
            in("rdi") pid,
            in("rsi") status,
            in("rdx") options,
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_getcwd(buf: &mut [u8]) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_GETCWD => ret,
            in("rdi") buf.as_mut_ptr(),
            in("rsi") buf.len(),
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

fn sys_chdir(path: *const u8) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_CHDIR => ret,
            in("rdi") path,
            out("rcx") _,
            out("r11") _,
        );
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void {
    let s = s as *mut u8;
    let mut i = 0;
    while i < n {
        unsafe { core::ptr::write_volatile(s.add(i), c as u8) };
        i += 1;
    }
    s as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    copy_bytes(dest as *mut u8, src as *const u8, n);
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    copy_bytes(dest as *mut u8, src as *const u8, n);
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int {
    let s1 = s1 as *const u8;
    let s2 = s2 as *const u8;
    let mut i = 0;
    while i < n {
        let a = unsafe { core::ptr::read_volatile(s1.add(i)) };
        let b = unsafe { core::ptr::read_volatile(s2.add(i)) };
        if a != b {
            return (a as c_int) - (b as c_int);
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const i8) -> usize {
    let s = s.cast::<u8>();
    let mut len = 0;
    while unsafe { core::ptr::read_volatile(s.add(len)) } != 0 {
        len += 1;
    }
    len
}

/// `copy_bytes` must handle overlapping regions because `memmove` forwards to it too,
/// so the copy direction depends on the relative position of the two buffers.
fn copy_bytes(dest: *mut u8, src: *const u8, n: usize) {
    if (dest as usize) <= (src as usize) || (dest as usize) >= (src as usize) + n {
        let mut i = 0;
        while i < n {
            unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))) };
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))) };
        }
    }
}

fn print(s: &str) {
    sys_write(1, s.as_bytes());
}

fn print_cstr(ptr: *const u8) {
    // `getcwd` leaves room for the terminating NUL but does not guarantee it, so stop at
    // the end of the buffer too instead of relying on the terminator being there.
    let mut len = 0;
    unsafe {
        while len < 128 && *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        sys_write(1, slice);
    }
}

fn cstr_eq(a: *const u8, b: *const u8) -> bool {
    let mut i = 0;
    unsafe {
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
    }
}