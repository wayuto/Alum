use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn syscall(nr: usize, a1: isize, a2: isize, a3: isize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") nr as isize,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            lateout("rax") ret,
            clobber_abi("C"),
        );
    }
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall6(
    nr: usize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
    a6: isize,
) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") nr as isize,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            in("r9") a6,
            lateout("rax") ret,
            clobber_abi("C"),
        );
    }
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn exit(code: isize) -> ! {
    unsafe {
        asm!(
            "mov rax, 60",
            "syscall",
            in("rdi") code,
            options(noreturn, nostack),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn getpid() -> isize {
    syscall(39, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn fork() -> isize {
    syscall(57, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    syscall(59, path as isize, argv as isize, envp as isize)
}

#[unsafe(no_mangle)]
pub extern "C" fn wait4(pid: isize, status: *mut u8, options: isize, rusage: *mut u8) -> isize {
    syscall6(61, pid, status as isize, options, rusage as isize, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn kill(pid: isize, sig: isize) -> isize {
    syscall(62, pid, sig, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn exit_group(code: isize) -> ! {
    unsafe {
        asm!(
            "mov rax, 231",
            "syscall",
            in("rdi") code,
            options(noreturn, nostack),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn getppid() -> isize {
    syscall(110, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn getuid() -> isize {
    syscall(102, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn geteuid() -> isize {
    syscall(107, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn getgid() -> isize {
    syscall(104, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn getegid() -> isize {
    syscall(108, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn sched_yield() -> isize {
    syscall(24, 0, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn mmap(
    addr: *const u8,
    length: usize,
    prot: isize,
    flags: isize,
    fd: isize,
    offset: isize,
) -> isize {
    syscall6(9, addr as isize, length as isize, prot, flags, fd, offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn munmap(addr: *const u8, length: usize) -> isize {
    syscall(11, addr as isize, length as isize, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn mprotect(addr: *const u8, length: usize, prot: isize) -> isize {
    syscall(10, addr as isize, length as isize, prot)
}

#[unsafe(no_mangle)]
pub extern "C" fn brk(addr: usize) -> usize {
    syscall(12, addr as isize, 0, 0) as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn nanosleep(req: *const u8, rem: *mut u8) -> isize {
    syscall(35, req as isize, rem as isize, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn clock_gettime(clk_id: isize, tp: *mut u8) -> isize {
    syscall(228, clk_id, tp as isize, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn gettimeofday(tv: *mut u8, tz: *mut u8) -> isize {
    syscall(96, tv as isize, tz as isize, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn open(path: *const u8, flags: isize, mode: isize) -> isize {
    syscall(2, path as isize, flags, mode)
}

#[unsafe(no_mangle)]
pub extern "C" fn close(fd: isize) -> isize {
    syscall(3, fd, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn pipe(pipefd: *mut u8) -> isize {
    syscall(22, pipefd as isize, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn pipe2(pipefd: *mut u8, flags: isize) -> isize {
    syscall(293, pipefd as isize, flags, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn dup(oldfd: isize) -> isize {
    syscall(32, oldfd, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn dup2(oldfd: isize, newfd: isize) -> isize {
    syscall(33, oldfd, newfd, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn dup3(oldfd: isize, newfd: isize, flags: isize) -> isize {
    syscall(292, oldfd, newfd, flags)
}

#[unsafe(no_mangle)]
pub extern "C" fn read(fd: isize, buf: *mut u8, n: usize) -> isize {
    syscall(0, fd, buf as isize, n as isize)
}

#[unsafe(no_mangle)]
pub extern "C" fn write(fd: isize, buf: *const u8, n: usize) -> isize {
    syscall(1, fd, buf as isize, n as isize)
}

#[unsafe(no_mangle)]
pub extern "C" fn lseek(fd: isize, off: isize, whence: isize) -> isize {
    syscall(8, fd, off, whence)
}

#[unsafe(no_mangle)]
pub extern "C" fn access(path: *const u8, mode: isize) -> isize {
    syscall(21, path as isize, mode, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn mkdir(path: *const u8, mode: isize) -> isize {
    syscall(83, path as isize, mode, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn rmdir(path: *const u8) -> isize {
    syscall(84, path as isize, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn unlink(path: *const u8) -> isize {
    syscall(87, path as isize, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn fsync(fd: isize) -> isize {
    syscall(74, fd, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn ftruncate(fd: isize, length: isize) -> isize {
    syscall(77, fd, length, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn getcwd(buf: *mut u8, size: usize) -> isize {
    syscall(79, buf as isize, size as isize, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn chdir(path: *const u8) -> isize {
    syscall(80, path as isize, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn getrandom(buf: *mut u8, len: usize, flags: isize) -> isize {
    syscall(318, buf as isize, len as isize, flags)
}

#[unsafe(no_mangle)]
pub extern "C" fn uname(buf: *mut u8) -> isize {
    syscall(63, buf as isize, 0, 0)
}
