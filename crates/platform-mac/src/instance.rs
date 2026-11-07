//! Single instance on macOS: a lock on a file in the data folder decides
//! which process is the primary one, and a Unix socket next to it lets later
//! launches hand it a message ([`SHOW`] or [`SETTINGS`], or a `loupe://`
//! link).
//!
//! The data folder is per user, so each user gets their own launcher. The
//! lock (`flock`) goes with the process, however it ends, so a crash leaves
//! nothing that stops the next launch; a socket file left behind is
//! replaced by the next primary instance.

use std::fs::File;

#[cfg(not(unix))]
pub use no_unix::*;
#[cfg(unix)]
pub use unix::*;

/// Asks the primary instance to show its launcher window.
pub const SHOW: &str = "show";

/// Asks the primary instance to open its settings window.
pub const SETTINGS: &str = "settings";

pub enum Instance {
    /// This process owns the launcher. Keep the guard alive for the process lifetime.
    Primary(Guard),
    /// Another process already does.
    Secondary,
}

/// Holds the lock; released when dropped (or when the process exits).
pub struct Guard(#[allow(dead_code)] Option<File>);

#[cfg(unix)]
mod unix {
    use std::{
        fs::{self, File},
        io::{self, Read, Write},
        os::{
            fd::AsRawFd,
            unix::net::{UnixListener, UnixStream},
        },
        path::PathBuf,
        thread,
    };

    use super::{Guard, Instance};
    use crate::{Result, paths};

    /// Longer messages are cut off; this is plenty for a command or a deeplink.
    const MAX_MESSAGE: usize = 4096;

    /// The longest path a Unix socket address holds on macOS (`sun_path`,
    /// with its terminating zero).
    const MAX_SOCKET_PATH: usize = 103;

    /// Decides whether this process is the primary instance.
    ///
    /// If the lock file can't be opened at all, this process acts as primary: a
    /// launcher that runs twice is better than one that never starts.
    pub fn acquire() -> Instance {
        let Some(dir) = paths::data_dir() else {
            eprintln!("[instance] no data folder, so no single-instance check");
            return Instance::Primary(Guard(None));
        };
        let opened = fs::create_dir_all(&dir).and_then(|()| {
            File::options()
                .create(true)
                .truncate(false)
                .write(true)
                .open(dir.join("loupe.lock"))
        });
        let file = match opened {
            Ok(file) => file,
            Err(err) => {
                eprintln!("[instance] could not open the lock file: {err}");
                return Instance::Primary(Guard(None));
            }
        };
        // `File::try_lock` would do, but it is newer than the crate's MSRV.
        const LOCK_EX: i32 = 2;
        const LOCK_NB: i32 = 4;
        unsafe extern "C" {
            fn flock(fd: i32, operation: i32) -> i32;
        }
        if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
            return Instance::Primary(Guard(Some(file)));
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            Instance::Secondary
        } else {
            eprintln!("[instance] could not lock the lock file: {err}");
            Instance::Primary(Guard(None))
        }
    }

    /// Primary instance: starts a thread that receives messages from later
    /// launches and forwards each one to `messages`.
    pub fn listen(messages: async_channel::Sender<String>) {
        let Some(path) = socket_path() else {
            eprintln!("[instance] no place for the instance socket");
            return;
        };
        // This process holds the lock, so a socket file there is left over
        // from an instance that is gone.
        let _ = fs::remove_file(&path);
        // The data folder is the user's own, so only they can connect.
        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("[instance] could not create the instance socket: {err}");
                return;
            }
        };
        let spawned = thread::Builder::new()
            .name("instance-socket".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    if let Some(message) = receive(stream)
                        && messages.send_blocking(message).is_err()
                    {
                        return;
                    }
                }
            });
        if let Err(err) = spawned {
            eprintln!("[instance] could not start the socket thread: {err}");
        }
    }

    /// Secondary instance: delivers `message` to the primary one.
    ///
    /// Also lets the primary instance take the foreground: this process may
    /// have just been made active (it was launched), the primary one wasn't.
    pub fn send(message: &str) -> Result<()> {
        let path = socket_path().ok_or("no place for the instance socket")?;
        let mut stream = UnixStream::connect(&path)?;
        #[cfg(target_os = "macos")]
        if let Some(pid) = peer_pid(&stream) {
            appkit::yield_activation_to(pid);
        }
        stream.write_all(message.as_bytes())?;
        Ok(())
    }

    /// Reads one client's whole message.
    fn receive(stream: UnixStream) -> Option<String> {
        let mut message = Vec::new();
        stream
            .take(MAX_MESSAGE as u64)
            .read_to_end(&mut message)
            .ok()?;
        String::from_utf8(message).ok()
    }

    /// `loupe.sock` in the data folder, or in the user's temporary folder when
    /// that path is too long for a socket address.
    fn socket_path() -> Option<PathBuf> {
        let fits = |path: &PathBuf| path.as_os_str().len() <= MAX_SOCKET_PATH;
        paths::data_dir()
            .map(|dir| dir.join("loupe.sock"))
            .filter(fits)
            .or_else(|| {
                // `TMPDIR` is per user on macOS (`/var/folders/…`).
                let tmp = std::env::var_os("TMPDIR")?;
                Some(PathBuf::from(tmp).join("dev.frgttn.loupe.sock")).filter(fits)
            })
    }

    /// The process at the other end of `stream`.
    #[cfg(target_os = "macos")]
    fn peer_pid(stream: &UnixStream) -> Option<i32> {
        use std::os::fd::AsRawFd;

        // `sys/un.h`: the level for Unix-domain socket options, and the option
        // that tells the peer's pid.
        const SOL_LOCAL: i32 = 0;
        const LOCAL_PEERPID: i32 = 2;
        unsafe extern "C" {
            fn getsockopt(
                socket: i32,
                level: i32,
                name: i32,
                value: *mut std::ffi::c_void,
                len: *mut u32,
            ) -> i32;
        }
        let mut pid: i32 = 0;
        let mut len = size_of::<i32>() as u32;
        let ok = unsafe {
            getsockopt(
                stream.as_raw_fd(),
                SOL_LOCAL,
                LOCAL_PEERPID,
                (&raw mut pid).cast(),
                &mut len,
            )
        } == 0;
        (ok && pid > 0).then_some(pid)
    }

    #[cfg(target_os = "macos")]
    mod appkit {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSRunningApplication};

        /// Lets the app with process id `pid` activate itself, if this process
        /// is active now. Since macOS 14 an app activates only when the active
        /// one lets it; when this process isn't active it has nothing to give,
        /// and this does nothing.
        // `unsafe` or not depending on the objc2 version; one block either way.
        #[allow(unused_unsafe)]
        pub fn yield_activation_to(pid: i32) {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let Some(primary) =
                (unsafe { NSRunningApplication::runningApplicationWithProcessIdentifier(pid) })
            else {
                return;
            };
            let app = NSApplication::sharedApplication(mtm);
            unsafe {
                app.yieldActivationToApplication(&primary);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_message_goes_through_the_socket() {
            let dir = std::env::temp_dir().join(format!("loupe-instance-{}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("test.sock");
            let _ = fs::remove_file(&path);
            let listener = UnixListener::bind(&path).unwrap();
            let mut client = UnixStream::connect(&path).unwrap();
            client.write_all(b"loupe://extensions").unwrap();
            drop(client);
            let (server, _) = listener.accept().unwrap();
            assert_eq!(receive(server).as_deref(), Some("loupe://extensions"));
            let _ = fs::remove_dir_all(&dir);
        }
    }
}

/// Without Unix sockets (building on Windows with `--cfg loupe_mac_stub`):
/// every launch is the primary one.
#[cfg(not(unix))]
mod no_unix {
    use super::{Guard, Instance};
    use crate::{Result, UNSUPPORTED};

    pub fn acquire() -> Instance {
        Instance::Primary(Guard(None))
    }
    pub fn listen(_messages: async_channel::Sender<String>) {}
    pub fn send(_message: &str) -> Result<()> {
        Err(UNSUPPORTED.into())
    }
}
