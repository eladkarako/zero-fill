use std::{fs::File, io, os::fd::AsRawFd, path::Path};

pub fn available_space(path: &Path) -> io::Result<u64> {
    let path = std::ffi::CString::new(path.to_string_lossy().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains an embedded NUL byte",
        )
    })?;

    let mut stats = unsafe { std::mem::zeroed::<libc::statvfs>() };

    let result = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };

    if result != 0 {
        return Err(io::Error::last_os_error());
    }

    (stats.f_bavail as u64)
        .checked_mul(stats.f_frsize as u64)
        .ok_or_else(|| io::Error::other("filesystem size overflow"))
}

pub fn fallocate(file: &File, offset: u64, length: u64) -> io::Result<()> {
    let result = unsafe {
        libc::fallocate(
            file.as_raw_fd(),
            0,
            offset as libc::off_t,
            length as libc::off_t,
        )
    };

    if result == 0 {
        return Ok(());
    }

    Err(io::Error::last_os_error())
}

pub fn is_interrupted(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::EINTR)
}

pub fn is_no_space(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ENOSPC)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
